//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

mod appearance;
mod command_palette;
mod context_menu;
mod inspector;
mod keybindings_tab;
mod library;
mod monitor;
mod presets;
mod scene;
mod toasts;
mod toggle_button;

pub use command_palette::{command_palette, PaletteOutcome};
pub(crate) use context_menu::context_menu;
pub use library::LibraryOutcome;
pub use monitor::cycle_entity_monitor;
pub use toasts::toasts;
pub use toggle_button::toggle_button;

use crate::asset_library::LibraryIndex;
use crate::i18n::t;
use crate::input::selection::SelectionState;
use crate::keybindings::KeyBindings;
use crate::monitor::{MonitorInfo, MonitorMode};
use crate::scene::Scene;
use crate::ui::banner::Warning;
use crate::ui::collapse::CollapseState;
use crate::ui::icons;
use crate::ui::onboarding::{self, OnboardingProgress};
use crate::ui::theme::{self, Theme, SPACE_S, SPACE_XS};

/// Entity-targeted action requested from the right-click context menu.
/// `App` applies it after `EguiRenderer::render` returns so it can grab a
/// mutable borrow on the renderer for texture management (Duplicate, Delete).
pub enum MenuAction {
    Duplicate(usize),
    Delete(usize),
    ResetTransform(usize),
    ToggleGravity(usize),
    BringForward(usize),
    SendBackward(usize),
}

/// What `context_menu` decided about its own state for this frame.
pub enum ContextMenuOutcome {
    /// Menu remains visible — nothing happened this frame.
    Open,
    /// User dismissed the menu (clicked outside).
    Close,
    /// User picked an action — caller should apply it and close the menu.
    Action(MenuAction),
}

/// Which tab is currently focused in the settings sidebar. Persisted
/// across frames in `egui::Memory` so we don't have to thread state
/// through `App` for purely UI-local switching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
enum SettingsTab {
    #[default]
    Inspector,
    Scene,
    Library,
    Appearance,
    Keybindings,
}

impl SettingsTab {
    const ALL: &'static [Self] = &[
        Self::Inspector,
        Self::Scene,
        Self::Library,
        Self::Appearance,
        Self::Keybindings,
    ];

    fn label(self) -> String {
        match self {
            Self::Inspector => t("settings-tab-inspector"),
            Self::Scene => t("settings-tab-scene"),
            Self::Library => t("settings-tab-library"),
            Self::Appearance => t("settings-tab-appearance"),
            Self::Keybindings => t("settings-tab-keybindings"),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Inspector => icons::CURSOR,
            Self::Scene => icons::STACK,
            Self::Library => icons::LIBRARY,
            Self::Appearance => icons::PALETTE,
            Self::Keybindings => icons::KEYBOARD,
        }
    }
}

/// Icon-only segmented tab bar, sized to stay uncluttered in the 320px
/// panel: the active tab gets an `accent_subtle` pill (`RADIUS_MD`) with
/// an `accent_base` glyph, the rest are `fg_muted` glyphs that tint on
/// hover. The human label rides along as a hover tooltip *and* the
/// AccessKit name (set via `widget_info`, so the icon-only bar stays
/// screen-reader-legible), and the caller paints it as the page title
/// just below. Returns the (possibly changed) active tab.
fn settings_tab_bar(ui: &mut egui::Ui, active: SettingsTab) -> SettingsTab {
    let palette = theme::palette_of(ui.ctx());
    let mut next = active;
    ui.columns(SettingsTab::ALL.len(), |cols| {
        for (col, &tab) in cols.iter_mut().zip(SettingsTab::ALL) {
            col.vertical_centered(|ui| {
                let is_active = tab == active;
                let size = egui::vec2(ui.available_width().min(44.0), 32.0);
                let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
                let radius = theme::RADIUS_MD as f32;
                if is_active {
                    ui.painter()
                        .rect_filled(rect, radius, palette.accent_subtle);
                } else if resp.hovered() {
                    ui.painter().rect_filled(rect, radius, palette.bg_elevated);
                }
                let color = if is_active {
                    palette.accent_base
                } else {
                    palette.fg_muted
                };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    tab.icon(),
                    egui::FontId::proportional(18.0),
                    color,
                );
                resp.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::SelectableLabel,
                        true,
                        is_active,
                        tab.label(),
                    )
                });
                if resp.on_hover_text(tab.label()).clicked() {
                    next = tab;
                }
            });
        }
    });
    next
}

/// A 1px full-width divider in the subtle border tone. Reads lighter than
/// egui's default `separator()` (which uses the heavier widget stroke),
/// for the cleaner panel rhythm.
fn hairline(ui: &mut egui::Ui) {
    let palette = theme::palette_of(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, palette.border_subtle);
}

/// Right-side settings panel. Organized into tabs (Inspector / Scene /
/// Library / Appearance / Keybindings) behind an icon-only tab bar, with
/// a sticky header and a scrollable body. Mutations flow directly through
/// the supplied mutable references; `config_dirty` is set when anything
/// changes so the save-on-exit-edit-mode path picks them up.
#[allow(clippy::too_many_arguments)]
pub fn settings(
    ctx: &egui::Context,
    open: bool,
    scene: &mut Scene,
    selection: &mut SelectionState,
    config_dirty: &mut bool,
    theme: &mut Theme,
    locale: &mut Option<String>,
    onboarding: &mut OnboardingProgress,
    monitor_mode: &mut MonitorMode,
    window_awareness: &mut bool,
    reduced_motion: &mut bool,
    monitors: &[MonitorInfo],
    library: Option<&LibraryIndex>,
    library_outcome: &mut Option<LibraryOutcome>,
    keybindings: &mut KeyBindings,
    collapse_state: &mut CollapseState,
    accesskit_enabled: &mut bool,
    warnings: &std::collections::BTreeSet<Warning>,
    last_seen_whats_new: &mut Option<String>,
    hotkey_backend: &str,
    shimeji_import: &mut Option<String>,
) {
    // Frosted, semi-transparent panel: the overlay feels lighter and the
    // desktop reads through behind the settings instead of a solid slab.
    // Alpha kept high (≈92%) so text contrast survives over a busy
    // background; a 1px subtle border gives the floating-card edge with
    // no heavy fill or shadow. Tunable — drop the alpha for more glass.
    let palette = theme::palette_of(ctx);
    let panel_frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(
            palette.bg_surface.r(),
            palette.bg_surface.g(),
            palette.bg_surface.b(),
            235,
        ))
        .inner_margin(egui::Margin::symmetric(theme::SPACE_M as i8, SPACE_S as i8))
        .stroke(egui::Stroke::new(1.0, palette.border_subtle));
    // `show_animated` slides the panel in/out with egui's animation
    // clock — which `ui::motion::set_reduced` zeroes under reduced
    // motion, so the slide collapses to an instant show/hide for free.
    egui::SidePanel::right("anima_settings")
        .resizable(false)
        .frame(panel_frame)
        .default_width(320.0)
        .show_animated(ctx, open, |ui| {
            // ── Sticky header ─────────────────────────────────────────
            ui.add_space(SPACE_S);
            ui.horizontal(|ui| {
                ui.add_space(SPACE_XS);
                ui.label(
                    egui::RichText::new(format!("{}  {}", icons::GHOST, t("app-name")))
                        .text_style(egui::TextStyle::Heading),
                );
            });
            ui.add_space(SPACE_S);
            hairline(ui);
            ui.add_space(SPACE_S);

            // ── Tab switcher (icon-only segmented bar) ────────────────
            let mut active_tab: SettingsTab = ui.memory(|m| {
                m.data
                    .get_temp::<SettingsTab>(egui::Id::new("anima.settings.tab"))
                    .unwrap_or_default()
            });
            active_tab = settings_tab_bar(ui, active_tab);
            ui.memory_mut(|m| {
                m.data
                    .insert_temp(egui::Id::new("anima.settings.tab"), active_tab);
            });

            // Active tab name as the panel's page title — the bar stays
            // icon-only to keep the narrow panel uncluttered — then a
            // hairline into the content.
            ui.add_space(SPACE_S);
            ui.horizontal(|ui| {
                ui.add_space(SPACE_XS);
                ui.label(egui::RichText::new(active_tab.label()).text_style(theme::h2()));
            });
            ui.add_space(SPACE_XS);
            hairline(ui);

            // ── Banners (session-lifetime warnings) ──────────────────
            // Rendered between the tab switcher and the tab body so
            // the user notices them the moment the panel opens; they
            // never appear in pass-through mode (the whole panel is
            // hidden there). Auto-disappear when the underlying
            // condition clears (see App::clear_warning).
            if !warnings.is_empty() {
                for warning in warnings {
                    appearance::warning_banner(ui, *warning);
                }
                ui.add_space(SPACE_XS);
            }

            // ── What's new panel (D.7) ───────────────────────────────
            // One-shot per minor-version bump; dismissing stamps the
            // current WHATS_NEW_VERSION into the config so the next
            // session skips the panel.
            if crate::ui::whats_new::show(ui, last_seen_whats_new) {
                *config_dirty = true;
            }

            // First-run hint right under the tab switcher.
            if onboarding::hint(ui, &t("onboarding-tabs"), &mut onboarding.tabs) {
                *config_dirty = true;
            }
            ui.add_space(SPACE_XS);

            // ── Tab body ──────────────────────────────────────────────
            // Each tab gets its own animate-value id so switching
            // restarts the curve from 0 and produces a 100ms fade-in
            // (linear, per design-system §6 "tab content cross-fade").
            let tab_alpha = ctx.animate_value_with_time(
                egui::Id::new(("anima.settings.tab.alpha", active_tab)),
                1.0,
                crate::ui::motion::time(ctx, 0.1),
            );
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_opacity(tab_alpha);
                    match active_tab {
                        SettingsTab::Inspector => {
                            inspector::inspector_tab(
                                ui,
                                scene,
                                selection,
                                config_dirty,
                                onboarding,
                                monitors,
                                collapse_state,
                            );
                        }
                        SettingsTab::Scene => {
                            scene::scene_tab(
                                ui,
                                scene,
                                selection,
                                config_dirty,
                                monitor_mode,
                                window_awareness,
                                monitors,
                                collapse_state,
                            );
                        }
                        SettingsTab::Library => {
                            library::library_tab(ui, library, library_outcome, shimeji_import);
                        }
                        SettingsTab::Appearance => {
                            appearance::appearance_tab(
                                ui,
                                theme,
                                locale,
                                config_dirty,
                                onboarding,
                                accesskit_enabled,
                                reduced_motion,
                            );
                        }
                        SettingsTab::Keybindings => {
                            // First-time hint about the rebinding UX (D.7).
                            if onboarding::hint(
                                ui,
                                &t("onboarding-keybindings"),
                                &mut onboarding.keybindings_tab,
                            ) {
                                *config_dirty = true;
                            }
                            keybindings_tab::keybindings_tab(
                                ctx,
                                ui,
                                keybindings,
                                config_dirty,
                                hotkey_backend,
                            );
                        }
                    }
                });

            // ── Sticky footer with entity count ───────────────────────
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                ui.add_space(SPACE_S);
                let count = scene.entities.len();
                let label = monitor::entity_count_label(count);
                ui.horizontal(|ui| {
                    ui.add_space(SPACE_XS);
                    ui.label(
                        egui::RichText::new(label)
                            .text_style(theme::caption())
                            .weak(),
                    );
                });
            });
        });
}
