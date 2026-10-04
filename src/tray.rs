//! System tray icon + menu.
//!
//! `ksni` implements the freedesktop StatusNotifierItem D-Bus protocol so
//! we don't need libappindicator (and the gtk/glib system deps it would
//! pull in). The tray lives on its own thread driving an `async-io`
//! executor; menu activations route back to whichever loop is running —
//! winit's or the native Wayland one — through an [`EventSink`].
//!
//! The menu is static but for snoozing (`tray_menu::entries`), whose
//! state lives in a process-wide atomic the tray thread looks at every
//! few seconds, asking ksni to show the menu again when it changed.

use crate::event::{AnimaEvent, EventSink};
use ksni::TrayMethods;
use std::thread;

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
            description: "Right-click for options".into(),
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
                        // looks every few seconds whether snoozing began or
                        // ended, so the menu shows it (`tray_menu::entries`).
                        let mut shown = crate::snooze::until_label();
                        loop {
                            async_io::Timer::after(std::time::Duration::from_secs(3)).await;
                            let now = crate::snooze::until_label();
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
