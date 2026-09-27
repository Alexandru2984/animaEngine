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
