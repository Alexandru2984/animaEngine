//! Desktop notifications (1.5, off by default: `notify_reminders`): a
//! reminder or a turn of the focus timer also goes to the desktop's
//! notifications. With the characters out of sight — hidden, snoozed,
//! stepped aside for a full-screen app — it goes there at once instead of
//! waiting for them (`crate::reminders::deliver`, `crate::focus::say_news`).
//!
//! Sent from a thread of its own, one at a time, so the loops never wait
//! on the bus: `org.freedesktop.Notifications` when installed natively,
//! the Notification portal inside the Flatpak, which needs no permission
//! beyond the portal. Nothing elsewhere yet. A failure is logged once; a
//! queue that backs up drops what comes after, rather than grow.

/// Show `body` under `title` on the desktop, soon.
pub fn send(title: &str, body: &str) {
    // Tests only note it: a test run must never reach the desktop of
    // whoever runs it.
    #[cfg(test)]
    if let Ok(mut sent) = SENT.lock() {
        sent.push((title.to_string(), body.to_string()));
        return;
    }
    #[cfg(all(unix, not(test)))]
    unix::send(title, body);
    #[cfg(not(all(unix, not(test))))]
    let _ = (title, body);
}

/// `text` escaped for a notification body, which a server may read as
/// markup (the Desktop Notifications spec): `&`, `<` and `>` as entities.
#[cfg(any(unix, test))]
fn plain_as_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// What tests sent.
#[cfg(test)]
pub(crate) static SENT: std::sync::Mutex<Vec<(String, String)>> = std::sync::Mutex::new(Vec::new());

#[cfg(all(unix, not(test)))]
mod unix {
    use std::collections::HashMap;
    use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
    use std::sync::{Mutex, OnceLock};
    use zbus::zvariant::Value;

    /// Notifications waiting for the thread, at most.
    const QUEUE: usize = 16;

    /// The way to the thread: a title and a body at a time.
    type Queue = Mutex<SyncSender<(String, String)>>;

    static SENDER: OnceLock<Option<Queue>> = OnceLock::new();

    pub fn send(title: &str, body: &str) {
        let sender = SENDER.get_or_init(|| {
            let (tx, rx) = sync_channel::<(String, String)>(QUEUE);
            std::thread::Builder::new()
                .name("anima-notify".into())
                .spawn(move || run(rx))
                .map_err(|e| tracing::warn!("Notifications: no thread: {e}"))
                .ok()
                .map(|_| Mutex::new(tx))
        });
        let Some(Ok(tx)) = sender.as_ref().map(|m| m.lock()) else {
            return;
        };
        match tx.try_send((title.to_string(), body.to_string())) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => tracing::debug!("Notifications: queue full, one dropped"),
            Err(TrySendError::Disconnected(_)) => {}
        }
    }

    fn run(rx: std::sync::mpsc::Receiver<(String, String)>) {
        let conn = match async_io::block_on(zbus::Connection::session()) {
            Ok(conn) => conn,
            Err(e) => {
                tracing::warn!("Notifications: no session bus: {e}");
                return;
            }
        };
        let portal = crate::autostart::in_flatpak();
        let mut warned = false;
        for (n, (title, body)) in (1u64..).zip(rx) {
            let sent = async_io::block_on(async {
                if portal {
                    let mut note: HashMap<&str, Value<'_>> = HashMap::new();
                    note.insert("title", Value::from(title.as_str()));
                    note.insert("body", Value::from(body.as_str()));
                    conn.call_method(
                        Some("org.freedesktop.portal.Desktop"),
                        "/org/freedesktop/portal/desktop",
                        Some("org.freedesktop.portal.Notification"),
                        "AddNotification",
                        &(format!("anima-{n}"), note),
                    )
                    .await
                    .map(|_| ())
                } else {
                    // The body is markup to a server that says it takes
                    // markup, and these are plain words: a reminder from a
                    // restored backup, a character's name from a shared
                    // scene, carried `<a href=…>` or `<img src=…>` through
                    // as markup, and an `&` broke it.
                    let body = super::plain_as_markup(&body);
                    let mut hints: HashMap<&str, Value<'_>> = HashMap::new();
                    hints.insert("desktop-entry", Value::from("com.animaengine.Anima"));
                    conn.call_method(
                        Some("org.freedesktop.Notifications"),
                        "/org/freedesktop/Notifications",
                        Some("org.freedesktop.Notifications"),
                        "Notify",
                        &(
                            "animaEngine",
                            0u32,
                            "anima-engine",
                            title.as_str(),
                            body.as_str(),
                            Vec::<&str>::new(),
                            hints,
                            -1i32,
                        ),
                    )
                    .await
                    .map(|_| ())
                }
            });
            if let Err(e) = sent {
                if !warned {
                    tracing::warn!("Notifications: not shown: {e}");
                    warned = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn plain_words_stay_plain_words_in_a_notification() {
        assert_eq!(
            super::plain_as_markup("Tom & Jerry <a href='x'>here</a>"),
            "Tom &amp; Jerry &lt;a href='x'&gt;here&lt;/a&gt;"
        );
        assert_eq!(super::plain_as_markup("Stretch, Ghost!"), "Stretch, Ghost!");
    }
}
