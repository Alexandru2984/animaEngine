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

// Service name kept in sync with the Flatpak app-id and the .desktop
// file id (see data/com.animaengine.Anima.metainfo.xml and the Flatpak
// manifest at flatpak/com.animaengine.Anima.yml). Flatpak only lets a
// sandboxed app own the bus name that matches its app-id.
const SERVICE_NAME: &str = "com.animaengine.Anima";
const OBJECT_PATH: &str = "/com/animaengine/Anima";
// The D-Bus *interface* the methods live on. Distinct from the bus name
// above: the bus name must match the Flatpak app-id (`com.…`), but the
// interface keeps the conventional reverse-DNS `org.…`. These are NOT
// interchangeable — a proxy built with the wrong one calls a
// non-existent interface and the method silently fails. Must stay equal
// to the `#[interface(name = …)]` literals below (attribute macros
// can't read a const, so the literals are spelled out there).
const INTERFACE_NAME: &str = "org.animaengine.Anima";

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

    /// Toggle edit-mode click-through. Mapped to the X11 path's
    /// `Ctrl+Shift+A` global hotkey; on Wayland-native sessions a
    /// compositor binding can call this via `gdbus`.
    async fn toggle_edit_mode(&self) {
        let _ = self.proxy.send_event(AnimaEvent::ToggleEditMode);
    }

    /// Hide the whole overlay. Pairs with `show_overlay`.
    async fn hide_overlay(&self) {
        let _ = self.proxy.send_event(AnimaEvent::HideOverlay);
    }

    /// Show a previously hidden overlay.
    async fn show_overlay(&self) {
        let _ = self.proxy.send_event(AnimaEvent::ShowOverlay);
    }

    /// Pause / resume every animation in the scene.
    async fn toggle_global_playback(&self) {
        let _ = self.proxy.send_event(AnimaEvent::ToggleGlobalPlayback);
    }
}

/// Wayland-flavoured twin of `ActivationService`. The native Wayland
/// run-loop doesn't go through winit's event loop, so the dispatch
/// target is an [`std::sync::mpsc::SyncSender<AnimaEvent>`] (bounded;
/// see [`DBUS_QUEUE_CAP`]) the main thread polls each frame. F.3
/// (0.5.1) replaced an unbounded channel + `send` with the bounded
/// pair + `try_send` so a spammy caller drops overflow events
/// instead of growing memory.
struct WaylandActivationService {
    tx: std::sync::mpsc::SyncSender<AnimaEvent>,
}

#[interface(name = "org.animaengine.Anima")]
impl WaylandActivationService {
    async fn activate(&self) {
        Self::dispatch(&self.tx, AnimaEvent::RaiseWindow, "Activate");
    }

    async fn toggle_edit_mode(&self) {
        Self::dispatch(&self.tx, AnimaEvent::ToggleEditMode, "ToggleEditMode");
    }

    async fn hide_overlay(&self) {
        Self::dispatch(&self.tx, AnimaEvent::HideOverlay, "HideOverlay");
    }

    async fn show_overlay(&self) {
        Self::dispatch(&self.tx, AnimaEvent::ShowOverlay, "ShowOverlay");
    }

    async fn toggle_global_playback(&self) {
        Self::dispatch(
            &self.tx,
            AnimaEvent::ToggleGlobalPlayback,
            "ToggleGlobalPlayback",
        );
    }
}

impl WaylandActivationService {
    /// Centralised try-send so each method has identical drop-on-full
    /// behaviour and a debug log when overflow happens. G.6 (0.5.3)
    /// added the log so a deluged operator can see in trace output why
    /// their `gdbus` calls aren't taking effect — without escalating
    /// to `warn` (that would let an attacker amplify their flood into
    /// our own log volume).
    fn dispatch(tx: &std::sync::mpsc::SyncSender<AnimaEvent>, ev: AnimaEvent, name: &'static str) {
        if let Err(e) = tx.try_send(ev) {
            tracing::debug!("D-Bus `{name}` dropped (queue full): {e}");
        }
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

/// Cap on the in-flight D-Bus action queue (F.3, 0.5.1). Pre-fix the
/// channel was unbounded; a malicious local process could call
/// `ToggleEditMode` in a tight loop and grow memory until the main
/// loop drained it next frame. With `sync_channel(64)` the sender's
/// `try_send` drops overflow events instead of blocking. 64 is an
/// order of magnitude past anything a real user produces in one
/// frame at 60 Hz.
const DBUS_QUEUE_CAP: usize = 64;

/// Mount the activation service for the native Wayland path. Returns
/// the receiver the run-loop polls each frame to consume action
/// events. The thread holds the connection alive so the service stays
/// reachable for `gdbus call` invocations from compositor bindings.
pub fn install_wayland_service(
    connection: zbus::Connection,
) -> std::sync::mpsc::Receiver<AnimaEvent> {
    let (tx, rx) = std::sync::mpsc::sync_channel::<AnimaEvent>(DBUS_QUEUE_CAP);
    // Spawn failure (e.g. RLIMIT_NPROC exhaustion — the same condition
    // F.2's thread-spawn hardening elsewhere in the codebase guards
    // against) degrades to "no D-Bus activation service" instead of
    // crashing the whole process at startup: `tx` drops with the
    // never-run closure, so `rx` just reports disconnected and the
    // run loop's `try_recv` loop already treats that as "nothing to
    // do" rather than an error worth propagating.
    if let Err(e) = std::thread::Builder::new()
        .name("anima-instance-wayland".into())
        .spawn(move || {
            async_io::block_on(async move {
                let service = WaylandActivationService { tx };
                if let Err(e) = connection.object_server().at(OBJECT_PATH, service).await {
                    tracing::warn!("Failed to publish Wayland activation service: {e}");
                    return;
                }
                let _conn = connection;
                std::future::pending::<()>().await;
            });
        })
    {
        tracing::warn!("Failed to spawn Wayland activation service thread: {e}");
    }
    rx
}

/// Mount the `Activate` method onto the connection and keep it alive on a
/// detached thread. Call only when `try_acquire` returned
/// `Claimed(Some(connection))` AND the event loop proxy is available.
pub fn install_service(connection: zbus::Connection, proxy: EventLoopProxy<AnimaEvent>) {
    // Same rationale as `install_wayland_service`: a spawn failure
    // here used to panic the whole process at startup on resource
    // exhaustion. Degrading to "no activation service" means a
    // second launch won't be able to raise this instance's window,
    // but the instance that's already running keeps running.
    if let Err(e) = std::thread::Builder::new()
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
    {
        tracing::warn!("Failed to spawn single-instance activation thread: {e}");
    }
}

/// Tell the existing primary instance to raise its window.
async fn signal_existing(connection: &zbus::Connection) {
    let proxy = match zbus::Proxy::new(connection, SERVICE_NAME, OBJECT_PATH, INTERFACE_NAME).await
    {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The bus name and the interface name are deliberately different
    /// namespaces (`com.…` app-id vs `org.…` interface). A regression
    /// once shipped that passed the bus name where `Proxy::new` wants the
    /// interface, so the second-instance `Activate` handoff called a
    /// non-existent interface and silently failed. Pin both so a future
    /// "tidy-up" that reunifies them is caught here, not in the field.
    #[test]
    fn bus_name_and_interface_are_distinct() {
        assert_eq!(SERVICE_NAME, "com.animaengine.Anima");
        assert_eq!(INTERFACE_NAME, "org.animaengine.Anima");
        assert_ne!(
            SERVICE_NAME, INTERFACE_NAME,
            "the proxy interface must be INTERFACE_NAME, not the bus name"
        );
    }

    /// The object path is the bus name with dots as slashes, leading `/`.
    #[test]
    fn object_path_matches_bus_name() {
        assert_eq!(OBJECT_PATH, "/com/animaengine/Anima");
    }
}
