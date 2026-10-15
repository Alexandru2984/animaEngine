//! System tray icon + menu.
//!
//! `ksni` implements the freedesktop StatusNotifierItem D-Bus protocol so
//! we don't need libappindicator (and the gtk/glib system deps it would
//! pull in). The tray lives on its own thread driving an `async-io`
//! executor; menu activations route back to whichever loop is running —
//! winit's or the native Wayland one — through an [`EventSink`].
//!
//! The menu changes with snoozing and the focus timer, whose state is
//! process-wide, and with the app's language (`tray_menu::entries`): the
//! tray thread looks at its labels every few seconds and asks ksni to
//! show the menu again when they changed.

use crate::event::{AnimaEvent, EventSink};
use ksni::TrayMethods;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::task::{Poll, Waker};
use std::thread;

/// Raised by a click in the menu, for the tray thread's loop below.
/// ksni shows the menu again right after a click, but the click's effect
/// lands in the app's loop a moment later, so that menu read as before
/// it: snoozed from the tray, it still offered to snooze, for up to the
/// loop's three seconds. Raised, the loop looks again at once.
struct Clicked {
    raised: AtomicBool,
    waker: Mutex<Option<Waker>>,
}

static CLICKED: Clicked = Clicked {
    raised: AtomicBool::new(false),
    waker: Mutex::new(None),
};

impl Clicked {
    fn raise(&self) {
        self.raised.store(true, Ordering::Release);
        if let Some(waker) = self.waker.lock().ok().and_then(|mut w| w.take()) {
            waker.wake();
        }
    }

    async fn wait(&self) {
        futures_lite::future::poll_fn(|cx| {
            if self.raised.swap(false, Ordering::AcqRel) {
                return Poll::Ready(());
            }
            if let Ok(mut waker) = self.waker.lock() {
                *waker = Some(cx.waker().clone());
            }
            // Raised between the look and the waker being stored.
            if self.raised.swap(false, Ordering::AcqRel) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

/// Tray + menu — owned and updated only on the tray thread.
struct AnimaTray {
    sink: EventSink,
}

impl ksni::Tray for AnimaTray {
    fn id(&self) -> String {
        "anima_engine".into()
    }

    fn icon_name(&self) -> String {
        // Standard freedesktop icon present on every modern desktop. Swap
        // for a custom raster once we settle on a brand mark.
        "applications-graphics".into()
    }

    fn title(&self) -> String {
        crate::tray_menu::TITLE.into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: crate::tray_menu::TITLE.into(),
            description: crate::i18n::t("tray-tooltip"),
            ..Default::default()
        }
    }

    /// Default activation (single click on KDE, double click on GNOME).
    fn activate(&mut self, _x: i32, _y: i32) {
        self.sink.send(crate::tray_menu::ACTIVATE);
    }

    /// Rendered from `tray_menu::entries`, which the Windows tray shares.
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use crate::tray_menu::Entry;
        use ksni::menu::*;
        crate::tray_menu::entries()
            .into_iter()
            .map(|entry| match entry {
                Entry::Separator => MenuItem::Separator,
                Entry::Item { label, event } => StandardItem {
                    label,
                    icon_name: match event {
                        Some(AnimaEvent::Quit) => "application-exit".into(),
                        _ => String::new(),
                    },
                    enabled: event.is_some(),
                    activate: Box::new(move |this: &mut Self| {
                        if let Some(event) = event {
                            this.sink.send(event);
                            CLICKED.raise();
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            })
            .collect()
    }
}

/// Spawn the tray on a dedicated thread. The returned `JoinHandle` is
/// detached intentionally — the tray dies when the process exits.
///
/// Failures to register with D-Bus are logged but don't abort startup;
/// the app remains usable from the toggle button and keybinds.
pub fn spawn(sink: EventSink) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("anima-tray".into())
        .spawn(move || {
            async_io::block_on(async move {
                let tray = AnimaTray { sink };
                // A tray item normally owns a per-process bus name,
                // `org.kde.StatusNotifierItem-PID-N`. Flatpak's bus proxy
                // lets an app own only the names its manifest declares, so
                // inside one the registration failed and there was no tray.
                // There the item registers under the connection's unique
                // name instead, as ksni documents and Chromium does.
                let in_flatpak = std::path::Path::new("/.flatpak-info").exists();
                match tray.disable_dbus_name(in_flatpak).spawn().await {
                    Ok(handle) => {
                        tracing::info!("System tray registered (StatusNotifierItem)");
                        // ksni keeps a server task alive on its own; this
                        // thread stays to keep the D-Bus connection, and
                        // looks every few seconds whether the menu would
                        // read differently now — snoozing or the focus
                        // timer began or ended, the language changed — so
                        // it shows that (`tray_menu::labels`). It used to
                        // look at snoozing only: the focus timer's start
                        // and stop never reached the menu.
                        let mut shown = crate::tray_menu::labels();
                        loop {
                            let clicked = futures_lite::future::or(
                                async {
                                    async_io::Timer::after(std::time::Duration::from_secs(3)).await;
                                    false
                                },
                                async {
                                    CLICKED.wait().await;
                                    true
                                },
                            )
                            .await;
                            if clicked {
                                // The click's effect, a frame or so away.
                                for _ in 0..15 {
                                    async_io::Timer::after(std::time::Duration::from_millis(100))
                                        .await;
                                    if crate::tray_menu::labels() != shown {
                                        break;
                                    }
                                }
                            }
                            let now = crate::tray_menu::labels();
                            if now != shown {
                                shown = now;
                                handle.update(|_| {}).await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Tray unavailable ({e}). The app still works — use \
                             the ⚙ button or keybinds."
                        );
                    }
                }
            });
        })
        .expect("failed to spawn tray thread")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A click before the loop waits is not lost, and one while it waits
    /// wakes it — the window between looking and storing the waker
    /// included.
    #[test]
    fn a_click_wakes_the_loop_before_or_while_it_waits() {
        let clicked = Clicked {
            raised: AtomicBool::new(false),
            waker: Mutex::new(None),
        };
        clicked.raise();
        futures_lite::future::block_on(clicked.wait());
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                clicked.raise();
            });
            futures_lite::future::block_on(clicked.wait());
        });
        assert!(!clicked.raised.load(Ordering::Acquire), "taken by the wait");
    }
}
