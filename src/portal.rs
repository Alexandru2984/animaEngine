//! Shared plumbing for `org.freedesktop.portal.*` calls.
//!
//! Every portal method returns an `org.freedesktop.portal.Request` object
//! path, and its result arrives later as a `Response` signal on that path.
//! The path is predictable from our unique bus name and a `handle_token`,
//! so the subscription is made *before* the call, closing the race.
//!
//! Used by the GlobalShortcuts client (`hotkeys::portal`) and the file
//! chooser (`file_chooser`).

use futures_lite::StreamExt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use zbus::zvariant::{OwnedValue, Value};

/// Why a portal request produced no results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortalError {
    /// The user dismissed the portal's dialog.
    Cancelled,
    /// No portal, a bus error, a timeout or a backend error.
    Failed(String),
}

impl std::fmt::Display for PortalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortalError::Cancelled => f.write_str("cancelled by user"),
            PortalError::Failed(reason) => f.write_str(reason),
        }
    }
}

impl From<String> for PortalError {
    fn from(reason: String) -> Self {
        PortalError::Failed(reason)
    }
}

impl From<PortalError> for String {
    fn from(e: PortalError) -> Self {
        e.to_string()
    }
}

/// Unique, spec-legal handle token (`[A-Za-z0-9_]`). The portal uses
/// it to pre-compute the Request object path.
pub fn next_handle_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("anima_{}_{n}", std::process::id())
}

/// What every portal request starts from: the session bus, this
/// connection's name as the portal's Request paths spell it
/// ([`sanitize_sender`]), and a proxy for the portal's `interface`
/// (`org.freedesktop.portal.FileChooser`, `…Background`).
pub async fn desktop_portal(
    interface: &'static str,
) -> Result<(zbus::Connection, String, zbus::Proxy<'static>), PortalError> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("session bus: {e}"))?;
    register_host_app(&conn).await;
    let sender = sanitize_sender(
        conn.unique_name()
            .ok_or_else(|| "connection has no unique name".to_string())?
            .as_str(),
    );
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        interface,
    )
    .await
    .map_err(|e| format!("{interface} proxy: {e}"))?;
    Ok((conn, sender, proxy))
}

/// This app's id: its desktop file's name, and its bus name.
pub const APP_ID: &str = "com.animaengine.Anima";

/// Say which app this connection is, for the portals that serve no app
/// without an id (GlobalShortcuts) — first, before any other portal call
/// on `conn` (`org.freedesktop.host.portal.Registry`, xdg-desktop-portal
/// 1.19 and later).
///
/// Without it a host app's id is read from the systemd scope its desktop
/// puts it in, and a connection keeps the first answer. On GNOME the scope
/// came 4 ms after 1.5 asked for its shortcuts — it starts faster than
/// 0.9 did — so the portal said "An app id is required" and the shortcuts
/// fell back to an X11 grab, which holds only while an X window has the
/// keyboard. Not inside Flatpak, where the sandbox says who it is.
pub async fn register_host_app(conn: &zbus::Connection) -> Registered {
    if std::path::Path::new("/.flatpak-info").exists() {
        return Registered::NotOurs;
    }
    let options: HashMap<&str, Value<'_>> = HashMap::new();
    match conn
        .call_method(
            Some("org.freedesktop.portal.Desktop"),
            "/org/freedesktop/portal/desktop",
            Some("org.freedesktop.host.portal.Registry"),
            "Register",
            &(APP_ID, options),
        )
        .await
    {
        Ok(_) => {
            tracing::debug!("Registered with the desktop portal as {APP_ID}");
            Registered::Yes
        }
        Err(e) => {
            tracing::debug!("Portal registry: {e}");
            match e {
                zbus::Error::MethodError(name, _, _)
                    if matches!(
                        name.as_str(),
                        "org.freedesktop.DBus.Error.UnknownMethod"
                            | "org.freedesktop.DBus.Error.UnknownInterface"
                            | "org.freedesktop.DBus.Error.UnknownObject"
                    ) =>
                {
                    Registered::NoRegistry
                }
                _ => Registered::NotOurs,
            }
        }
    }
}

/// What [`register_host_app`] got.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Registered {
    /// The portal knows this connection by [`APP_ID`].
    Yes,
    /// A portal older than its registry: it reads the id from the app's
    /// systemd scope ([`wait_for_app_scope`]).
    NoRegistry,
    /// Not for us to say — inside a sandbox, which says it, or no portal
    /// at all.
    NotOurs,
}

/// Wait, up to `limit`, for the desktop to put this process in its app's
/// systemd scope — where a portal without the registry reads the app's id
/// from. Returns whether it is there. Started some other way (a terminal)
/// it never is, and this waits `limit` out.
pub async fn wait_for_app_scope(limit: std::time::Duration) -> bool {
    let until = std::time::Instant::now() + limit;
    loop {
        let cgroup = std::fs::read_to_string("/proc/self/cgroup").unwrap_or_default();
        if in_app_scope(&cgroup, APP_ID) {
            return true;
        }
        if std::time::Instant::now() >= until {
            return false;
        }
        async_io::Timer::after(std::time::Duration::from_millis(50)).await;
    }
}

/// Whether `/proc/self/cgroup` puts this process in a systemd unit of the
/// app `app_id`'s — `app-gnome-com.animaengine.Anima-4242.scope`, as the
/// desktop names the ones it launches (systemd's desktop-environment
/// convention).
fn in_app_scope(cgroup: &str, app_id: &str) -> bool {
    cgroup.lines().any(|line| {
        line.rsplit('/').next().is_some_and(|unit| {
            unit.starts_with("app-")
                && unit
                    .split(['-', '@'])
                    .any(|part| part == app_id || part.strip_suffix(".scope") == Some(app_id))
        })
    })
}

/// `:1.42` → `1_42`, per the portal Request-path convention.
pub fn sanitize_sender(unique_name: &str) -> String {
    unique_name
        .trim_start_matches(':')
        .replace('.', "_")
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Extract a string entry from a portal response vardict.
pub fn vardict_str(results: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let v = results.get(key)?;
    match &**v {
        Value::Str(s) => Some(s.to_string()),
        Value::ObjectPath(p) => Some(p.to_string()),
        _ => None,
    }
}

/// One portal request round-trip: subscribe on the predicted Request
/// path, fire the method, await the `Response` signal, return its
/// results vardict. `build_args` receives the generated handle_token
/// by value so the argument tuple owns every string it serializes.
pub async fn portal_request<A>(
    conn: &zbus::Connection,
    sender: &str,
    proxy: &zbus::Proxy<'_>,
    method: &str,
    timeout: Option<std::time::Duration>,
    build_args: impl FnOnce(String) -> A,
) -> Result<HashMap<String, OwnedValue>, PortalError>
where
    A: serde::Serialize + zbus::zvariant::DynamicType,
{
    let token = next_handle_token();
    let request_path = format!("/org/freedesktop/portal/desktop/request/{sender}/{token}");

    let request_proxy = zbus::Proxy::new(
        conn,
        "org.freedesktop.portal.Desktop",
        request_path.as_str(),
        "org.freedesktop.portal.Request",
    )
    .await
    .map_err(|e| format!("Request proxy: {e}"))?;
    let mut responses = request_proxy
        .receive_signal("Response")
        .await
        .map_err(|e| format!("subscribe Response: {e}"))?;

    proxy
        .call_method(method, &build_args(token.clone()))
        .await
        .map_err(|e| format!("{method}: {e}"))?;

    // `timeout` bounds the wait for requests that should answer at
    // once; a file chooser waits on a person and passes `None`.
    let next_fut = async { Some(responses.next().await) };
    let timeout_fut = async {
        match timeout {
            Some(t) => {
                async_io::Timer::after(t).await;
                None
            }
            None => std::future::pending().await,
        }
    };
    let msg = match futures_lite::future::or(next_fut, timeout_fut).await {
        None => return Err(format!("{method}: portal did not respond in time").into()),
        Some(None) => return Err(format!("{method}: response stream closed").into()),
        Some(Some(m)) => m,
    };
    let (code, results) = msg
        .body()
        .deserialize::<(u32, HashMap<String, OwnedValue>)>()
        .map_err(|e| format!("{method} response decode: {e}"))?;
    match code {
        0 => Ok(results),
        1 => Err(PortalError::Cancelled),
        other => Err(PortalError::Failed(format!(
            "{method}: portal error code {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_tokens_are_unique_and_legal() {
        let a = next_handle_token();
        let b = next_handle_token();
        assert_ne!(a, b);
        for t in [&a, &b] {
            assert!(
                t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'),
                "token {t} carries an illegal char"
            );
        }
    }

    #[test]
    fn the_apps_own_scope_is_told_from_others() {
        let ours = "0::/user.slice/user-1000.slice/user@1000.service/app.slice/\
                    app-gnome-com.animaengine.Anima-2892862.scope\n";
        assert!(in_app_scope(ours, APP_ID));
        assert!(in_app_scope(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-com.animaengine.Anima@x.service",
            APP_ID
        ));
        // Still in the shell's, a moment after it started us.
        assert!(!in_app_scope(
            "0::/user.slice/user-1000.slice/user@1000.service/session.slice/org.gnome.Shell@ubuntu.service",
            APP_ID
        ));
        // Started from a terminal in another app's scope.
        assert!(!in_app_scope(
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-gnome-code-1234.scope",
            APP_ID
        ));
        assert!(!in_app_scope("", APP_ID));
    }

    #[test]
    fn sender_sanitization_matches_portal_convention() {
        assert_eq!(sanitize_sender(":1.42"), "1_42");
        assert_eq!(sanitize_sender(":1.5-weird"), "1_5_weird");
    }

    #[test]
    fn vardict_reads_strings_and_object_paths() {
        let mut m: HashMap<String, OwnedValue> = HashMap::new();
        m.insert(
            "session_handle".into(),
            Value::from("/org/fdo/session/x").try_into().unwrap(),
        );
        assert_eq!(
            vardict_str(&m, "session_handle").as_deref(),
            Some("/org/fdo/session/x")
        );
        assert_eq!(vardict_str(&m, "missing"), None);
    }
}
