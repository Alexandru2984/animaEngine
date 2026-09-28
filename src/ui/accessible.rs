//! Names for screen readers where egui has none to give.
//!
//! egui names a widget after its text. An icon-only button's text is one
//! glyph from the icon font — a private-use character a screen reader
//! reads as nothing — and a combo box built without a label has no name
//! at all. These give each the name a sighted user gets from its tooltip
//! or from the label beside it. (Icons *next to* text are dropped from
//! names further down, in `crate::a11y`.)

/// Accessible names for `egui::Response`.
pub trait AccessibleName {
    /// Hover text that is also the control's name for screen readers.
    /// For icon-only buttons, whose glyph names nothing.
    fn on_hover_name(self, text: impl Into<String>) -> Self;

    /// The button's name for screen readers, when it has to say more than
    /// its tooltip: which of several identical icons this one is.
    fn named(self, name: impl Into<String>) -> Self;
}

impl AccessibleName for egui::Response {
    fn on_hover_name(self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.named(text.clone()).on_hover_text(text)
    }

    fn named(self, name: impl Into<String>) -> Self {
        let name = name.into();
        let enabled = self.enabled();
        self.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, &name));
        self
    }
}

/// Options in a combo box's list.
pub trait ComboOption {
    /// Whether the option was picked this frame; if it was, the list
    /// closes. egui closes it only on a pointer click: picked from the
    /// keyboard or by a screen reader, it stayed open over the panel.
    fn picked(&self) -> bool;
}

impl ComboOption for egui::Response {
    fn picked(&self) -> bool {
        let clicked = self.clicked();
        if clicked {
            self.ctx.memory_mut(|m| m.close_popup());
        }
        clicked
    }
}

/// Name a text field that has no label of its own, only an icon beside
/// it and a hint inside. A hint is not a name to a screen reader; the
/// field read as an unnamed entry. What was typed stays its value.
pub fn name_text_field(response: &egui::Response, name: &str) {
    let enabled = response.enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, enabled, name));
}

/// Name a combo box after its label, with its current choice as the
/// value — what a screen reader says on reaching it.
pub fn name_combo(response: &egui::Response, name: &str, value: &str) {
    let enabled = response.enabled();
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, enabled, name);
        info.current_text_value = Some(value.to_string());
        info
    });
}
