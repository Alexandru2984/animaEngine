//! The row of arrange buttons (1.5): line the selection up on an edge or
//! a centre, or space it evenly (`crate::input::arrange`). In the
//! Inspector and on the right-click menu, when several characters are
//! selected.

use crate::input::arrange::{Align, Arrange};
use crate::ui::accessible::AccessibleName;
use crate::ui::icons;

fn icon(how: Arrange) -> &'static str {
    match how {
        Arrange::Align(Align::Left) => icons::ALIGN_LEFT,
        Arrange::Align(Align::Center) => icons::ALIGN_CENTER,
        Arrange::Align(Align::Right) => icons::ALIGN_RIGHT,
        Arrange::Align(Align::Top) => icons::ALIGN_TOP,
        Arrange::Align(Align::Middle) => icons::ALIGN_MIDDLE,
        Arrange::Align(Align::Bottom) => icons::ALIGN_BOTTOM,
        Arrange::DistributeHorizontally => icons::DISTRIBUTE_HORIZONTALLY,
        Arrange::DistributeVertically => icons::DISTRIBUTE_VERTICALLY,
    }
}

/// The buttons for arranging `count` characters, those that need more
/// greyed out. Returns the one clicked.
pub(crate) fn arrange_buttons(ui: &mut egui::Ui, count: usize) -> Option<Arrange> {
    let mut picked = None;
    ui.horizontal(|ui| {
        for how in Arrange::ALL {
            if how == Arrange::DistributeHorizontally {
                ui.separator();
            }
            let name = crate::i18n::t(how.i18n_key());
            let button = ui
                .add_enabled(count >= how.needs(), egui::Button::new(icon(how)).small())
                .on_hover_name(name);
            if button.clicked() {
                picked = Some(how);
            }
        }
    });
    picked
}
