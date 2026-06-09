//! `PointerHandler` impl + Linux input button-code translation.
//! Extracted in K.3.
//!
//! sctk batches motion / press / release / axis into one frame; we
//! flatten each into an `egui::Event` so the integration code in
//! `LayerWindow::drain_egui_events` can just hand them to egui.
//!
//! Scroll axis is reported in two flavours by Wayland (`discrete`
//! steps for wheels, `absolute` pixels for touchpads). We use
//! `absolute` for fidelity — both events sources surface there and
//! egui's pixel-mode wheel is close enough to right without extra
//! scaling.

use super::state::WaylandState;
use crate::wayland::keyboard::modifiers_to_egui;
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerEventKind, PointerHandler};
use wayland_client::{protocol::wl_pointer, Connection, QueueHandle};

impl PointerHandler for WaylandState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            match &event.kind {
                PointerEventKind::Enter { .. } => {
                    self.cursor_pos = Some((event.position.0 as f32, event.position.1 as f32));
                    self.pending_egui_events
                        .push(egui::Event::PointerMoved(egui::pos2(
                            event.position.0 as f32,
                            event.position.1 as f32,
                        )));
                }
                PointerEventKind::Leave { .. } => {
                    self.cursor_pos = None;
                    self.pending_egui_events.push(egui::Event::PointerGone);
                }
                PointerEventKind::Motion { .. } => {
                    self.cursor_pos = Some((event.position.0 as f32, event.position.1 as f32));
                    self.pending_egui_events
                        .push(egui::Event::PointerMoved(egui::pos2(
                            event.position.0 as f32,
                            event.position.1 as f32,
                        )));
                }
                PointerEventKind::Press { button, .. } => {
                    if let Some(b) = linux_button_to_egui(*button) {
                        if let Some((x, y)) = self.cursor_pos {
                            self.pending_egui_events.push(egui::Event::PointerButton {
                                pos: egui::pos2(x, y),
                                button: b,
                                pressed: true,
                                modifiers: modifiers_to_egui(self.last_modifiers),
                            });
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if let Some(b) = linux_button_to_egui(*button) {
                        if let Some((x, y)) = self.cursor_pos {
                            self.pending_egui_events.push(egui::Event::PointerButton {
                                pos: egui::pos2(x, y),
                                button: b,
                                pressed: false,
                                modifiers: modifiers_to_egui(self.last_modifiers),
                            });
                        }
                    }
                }
                PointerEventKind::Axis {
                    horizontal,
                    vertical,
                    ..
                } => {
                    // Wayland reports scroll in "discrete steps" (mouse
                    // wheel) and "absolute" (touchpad). Use `absolute`
                    // for fidelity; egui expects pixels per second-ish.
                    let dx = -horizontal.absolute as f32;
                    let dy = -vertical.absolute as f32;
                    if dx != 0.0 || dy != 0.0 {
                        self.pending_egui_events.push(egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(dx, dy),
                            modifiers: egui::Modifiers::NONE,
                        });
                    }
                }
            }
        }
    }
}

/// Linux input event button codes (from `linux/input-event-codes.h`).
/// We only translate the three buttons egui actually handles.
fn linux_button_to_egui(code: u32) -> Option<egui::PointerButton> {
    match code {
        0x110 => Some(egui::PointerButton::Primary), // BTN_LEFT
        0x111 => Some(egui::PointerButton::Secondary), // BTN_RIGHT
        0x112 => Some(egui::PointerButton::Middle),  // BTN_MIDDLE
        _ => None,
    }
}
