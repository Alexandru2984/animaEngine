//! System tray icon + menu.
//!
//! `ksni` implements the freedesktop StatusNotifierItem D-Bus protocol so
//! we don't need libappindicator (and the gtk/glib system deps it would
//! pull in). The tray lives on its own thread driving an `async-io`
//! executor; menu activations route back to the winit event loop through
//! a `EventLoopProxy<AnimaEvent>`.
//!
//! The tray currently shows static labels — it does not reflect live
//! state changes from the UI thread. That's a deliberate simplification
//! to keep the cross-thread surface tiny; reflecting state would need an
//! async channel + per-update `handle.update()` await. Worth doing once
//! the menu earns it.

use crate::event::AnimaEvent;
use ksni::TrayMethods;
use std::thread;
use winit::event_loop::EventLoopProxy;

/// Tray + menu — owned and updated only on the tray thread.
struct AnimaTray {
    proxy: EventLoopProxy<AnimaEvent>,
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
        let _ = self.proxy.send_event(crate::tray_menu::ACTIVATE);
    }

    /// Rendered from `tray_menu::MENU`, which the Windows tray shares.
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use crate::tray_menu::{TrayItem, MENU};
        use ksni::menu::*;
        MENU.iter()
            .map(|item| match *item {
                TrayItem::Separator => MenuItem::Separator,
                TrayItem::Action { label, event } => StandardItem {
                    label: label.into(),
                    icon_name: match event {
                        AnimaEvent::Quit => "application-exit".into(),
                        _ => String::new(),
                    },
                    activate: Box::new(move |this: &mut Self| {
                        let _ = this.proxy.send_event(event);
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
pub fn spawn(proxy: EventLoopProxy<AnimaEvent>) -> thread::JoinHandle<()> {
    thread::Builder::new()
        .name("anima-tray".into())
        .spawn(move || {
            async_io::block_on(async move {
                let tray = AnimaTray { proxy };
                match tray.spawn().await {
                    Ok(_handle) => {
                        tracing::info!("System tray registered (StatusNotifierItem)");
                        // Park the executor — ksni keeps a server task alive
                        // on its own; we just need to keep this thread running
                        // so the D-Bus connection isn't dropped.
                        std::future::pending::<()>().await;
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
