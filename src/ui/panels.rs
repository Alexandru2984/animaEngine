//! UI panels rendered through egui. Each function takes an `&egui::Context`
//! plus any state it needs and emits widgets — no rendering plumbing here.

/// Tiny placeholder window that proves the wgpu + egui integration is
/// working end-to-end. Replaced by a real settings panel in 3.2.
pub fn edit_mode_probe(ctx: &egui::Context, entity_count: usize) {
    egui::Window::new("Anima")
        .resizable(false)
        .collapsible(true)
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            ui.label("Edit mode active");
            ui.separator();
            ui.label(format!("Entities: {entity_count}"));
            ui.label("More controls coming in 3.2");
        });
}
