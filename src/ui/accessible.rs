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
