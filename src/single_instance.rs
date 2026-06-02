//! Single-instance lock via D-Bus name ownership.
//!
//! At startup we try to claim the well-known name `org.animaengine.Anima`
//! on the session bus. If we win the claim we keep the connection alive
//! and expose an `Activate` method; if someone else already owns it, we
//! call `Activate` on them so they raise their window, then exit.
//!
//! Falls back to "no lock, proceed" when the user has no session bus
//! (rare on graphical Linux systems but possible in containers / CI).

use crate::event::AnimaEvent;
use winit::event_loop::EventLoopProxy;
use zbus::interface;
use zbus::names::WellKnownName;

const SERVICE_NAME: &str = "org.animaengine.Anima";
const OBJECT_PATH: &str = "/org/animaengine/Anima";

/// What the initial handshake decided about this process.
pub enum AcquireOutcome {
    /// We own the name (or there's no D-Bus at all). The optional connection
    /// must be handed to `install_service` after the event loop is built;
    /// dropping it would release the name and accept a future instance.
    Claimed(Option<zbus::Connection>),
    /// Another instance already owns the name and was pinged to raise.
    /// Caller should exit immediately.
    HandedOff,
}

/// Service exposed on the D-Bus. Activations by a second-launch instance
/// post `AnimaEvent::RaiseWindow` to the main thread.
struct ActivationService {
    proxy: EventLoopProxy<AnimaEvent>,
}

#[interface(name = "org.animaengine.Anima")]
impl ActivationService {
    async fn activate(&self) {
        let _ = self.proxy.send_event(AnimaEvent::RaiseWindow);
    }
}

/// Try to grab the single-instance lock. Synchronous wrapper around the
/// async D-Bus dance — uses `async_io` since it's already in our deps
/// (transitively through ksni).
pub fn try_acquire() -> AcquireOutcome {
    async_io::block_on(async {
        let connection = match zbus::Connection::session().await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("No D-Bus session ({e}). Single-instance disabled.");
                return AcquireOutcome::Claimed(None);
            }
        };

        // `DoNotQueue` means: don't put us on a waiting list if the name
        // is taken — fail fast instead so we can hand off and exit.
        use zbus::fdo::{RequestNameFlags, RequestNameReply};
        let name: WellKnownName = match SERVICE_NAME.try_into() {
            Ok(n) => n,
            Err(e) => {
                tracing::warn!("Bad service name (compile-time bug): {e}");
                return AcquireOutcome::Claimed(None);
            }
        };

        let reply = connection
            .request_name_with_flags(&name, RequestNameFlags::DoNotQueue.into())
            .await;

        match reply {
            Ok(RequestNameReply::PrimaryOwner) => {
                tracing::info!("Claimed single-instance name {SERVICE_NAME}");
                AcquireOutcome::Claimed(Some(connection))
            }
            Ok(RequestNameReply::Exists) | Ok(RequestNameReply::AlreadyOwner) => {
                signal_existing(&connection).await;
                AcquireOutcome::HandedOff
            }
            Ok(other) => {
                tracing::warn!("Unexpected RequestName reply: {other:?}. Proceeding.");
                AcquireOutcome::Claimed(Some(connection))
            }
            Err(e) => {
                tracing::warn!("RequestName failed ({e}). Proceeding without lock.");
                AcquireOutcome::Claimed(None)
            }
        }
    })
}

/// Mount the `Activate` method onto the connection and keep it alive on a
/// detached thread. Call only when `try_acquire` returned
/// `Claimed(Some(connection))` AND the event loop proxy is available.
pub fn install_service(connection: zbus::Connection, proxy: EventLoopProxy<AnimaEvent>) {
    std::thread::Builder::new()
        .name("anima-instance".into())
        .spawn(move || {
            async_io::block_on(async move {
                let service = ActivationService { proxy };
                if let Err(e) = connection.object_server().at(OBJECT_PATH, service).await {
                    tracing::warn!("Failed to publish activation service: {e}");
                    return;
                }
                // Hold the connection (and the name) for the rest of the
                // process. Dropping it would release the lock.
                let _conn = connection;
                std::future::pending::<()>().await;
            });
        })
        .expect("spawn single-instance thread");
}

/// Tell the existing primary instance to raise its window.
async fn signal_existing(connection: &zbus::Connection) {
    let proxy = match zbus::Proxy::new(connection, SERVICE_NAME, OBJECT_PATH, SERVICE_NAME).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Couldn't reach existing instance: {e}");
            return;
        }
    };
    match proxy.call_method("Activate", &()).await {
        Ok(_) => tracing::info!("Asked existing instance to raise its window"),
        Err(e) => tracing::warn!("Activate call failed: {e}"),
    }
}
