//! "Add file…": the desktop's own file chooser, through the XDG portal.
//!
//! Dragging a file onto the overlay is the other way in, and inside the
//! Flatpak it only works for files in Pictures and Downloads — anything
//! else is invisible to the sandbox, and the drop does nothing (R49). The
//! portal's chooser runs outside the sandbox and hands back exactly the
//! files picked, readable from inside it. It also gives a way in that
//! needs no dragging at all.
//!
//! No dialog library: `org.freedesktop.portal.FileChooser.OpenFile` over
//! the session bus, through the same request/response helper the global
//! shortcuts use (`crate::portal`). The call waits on a person, so it runs
//! on its own thread and the caller polls for the answer.

use std::path::PathBuf;
use std::sync::mpsc;

/// What the chooser came back with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Chosen {
    /// The files picked; empty if the portal returned none.
    Files(Vec<PathBuf>),
    /// The dialog was dismissed.
    Cancelled,
    /// No chooser could be shown — no portal on this session, a bus
    /// error, or a platform without one. The reason is for the log.
    Unavailable(String),
}

/// A chooser dialog that is open, or about to be.
pub struct FileChooser {
    rx: mpsc::Receiver<Chosen>,
}

impl FileChooser {
    /// Ask the desktop to show its file chooser. Returns at once; the
    /// answer arrives through [`FileChooser::poll`].
    ///
    /// `title` heads the dialog, `filter_name` labels the file-type
    /// filter, which offers exactly the formats a drop accepts.
    pub fn open(title: String, filter_name: String) -> Self {
        let (tx, rx) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("anima-file-chooser".into())
            .spawn(move || {
                let _ = tx.send(run(&title, &filter_name));
            });
        if let Err(e) = spawned {
            // The sender went with the closure, so `poll` reports the
            // chooser as unavailable through the disconnect.
            tracing::warn!("File chooser thread failed to start: {e}");
        }
        Self { rx }
    }

    /// The answer, once there is one. `None` while the dialog is up.
    pub fn poll(&self) -> Option<Chosen> {
        match self.rx.try_recv() {
            Ok(chosen) => Some(chosen),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                Some(Chosen::Unavailable("the chooser thread ended".into()))
            }
        }
    }
}

#[cfg(unix)]
fn run(title: &str, filter_name: &str) -> Chosen {
    use crate::portal::PortalError;
    match async_io::block_on(open_files(title, filter_name)) {
        Ok(paths) => Chosen::Files(paths),
        Err(PortalError::Cancelled) => Chosen::Cancelled,
        Err(PortalError::Failed(reason)) => Chosen::Unavailable(reason),
    }
}

#[cfg(not(unix))]
fn run(_title: &str, _filter_name: &str) -> Chosen {
    Chosen::Unavailable("no file chooser on this platform yet".into())
}

#[cfg(unix)]
async fn open_files(
    title: &str,
    filter_name: &str,
) -> Result<Vec<PathBuf>, crate::portal::PortalError> {
    use crate::portal::{portal_request, sanitize_sender};
    use std::collections::HashMap;
    use zbus::zvariant::Value;

    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("session bus: {e}"))?;
    let sender = sanitize_sender(
        conn.unique_name()
            .ok_or_else(|| "connection has no unique name".to_string())?
            .as_str(),
    );
    let proxy = zbus::Proxy::new(
        &conn,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.FileChooser",
    )
    .await
    .map_err(|e| format!("FileChooser proxy: {e}"))?;

    let filters = vec![(filter_name.to_string(), filter_patterns())];
    // No timeout: the dialog waits on a person.
    let results = portal_request(&conn, &sender, &proxy, "OpenFile", None, |token| {
        let mut options: HashMap<&str, Value<'_>> = HashMap::new();
        options.insert("handle_token", Value::from(token));
        options.insert("multiple", Value::from(true));
        options.insert("filters", Value::from(filters));
        // No parent window: a layer surface has no handle the portal
        // understands, and none is needed for the dialog to appear.
        ("", title.to_string(), options)
    })
    .await?;

    let uris: Vec<String> = match results.get("uris").map(|v| &**v) {
        Some(Value::Array(array)) => array
            .iter()
            .filter_map(|v| match v {
                Value::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    // The same parser as a Wayland drop's `text/uri-list`: `file://` only,
    // percent-decoded, capped.
    Ok(crate::wayland::data_device::parse_uri_list(
        uris.join("\r\n").as_bytes(),
    ))
}

/// Glob patterns for the formats a drop accepts, in both cases: the
/// portal's globs are case-sensitive on some backends.
#[cfg(unix)]
fn filter_patterns() -> Vec<(u32, String)> {
    crate::drop_validate::DROP_EXTENSIONS
        .iter()
        .flat_map(|ext| [ext.to_string(), ext.to_uppercase()])
        .map(|ext| (0u32, format!("*.{ext}")))
        .collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn the_filter_offers_every_drop_format_in_both_cases() {
        let patterns = filter_patterns();
        for ext in crate::drop_validate::DROP_EXTENSIONS {
            for form in [ext.to_string(), ext.to_uppercase()] {
                assert!(
                    patterns.contains(&(0, format!("*.{form}"))),
                    "*.{form} missing"
                );
            }
        }
        // Kind 0 is a glob; the portal also knows 1, a MIME type.
        assert!(patterns.iter().all(|(kind, _)| *kind == 0));
    }
}
