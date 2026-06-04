//! Legacy entry-point for the keyboard action table. The authoritative
//! dispatch and rebinding logic lives in [`crate::keybindings`]; this
//! module remains only so existing UI code can keep referring to
//! `ui::keyboard::Action`.

pub use crate::keybindings::Action;
