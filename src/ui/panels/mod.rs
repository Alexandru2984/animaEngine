//! UI panels rendered through egui.
//!
//! Each function takes only the data it actually mutates plus `&egui::Context`.
//! This keeps `App` borrow-safe: the caller passes disjoint `&mut` references
//! to scene / selection / dirty flag instead of `&mut self`.

mod appearance;
mod arrange;
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

pub use command_palette::{command_palette, command_palette_open, PaletteOutcome};
pub(crate) use context_menu::context_menu;
pub use library::LibraryOutcome;
pub use monitor::{cycle_entity_monitor, move_to_pinned_monitor};
pub use toasts::toasts;
pub use toggle_button::toggle_button;

/// Whether the panels hold the keyboard even with no text field focused:
/// the command palette while it is open, an open list (a combo box's),
/// and a shortcut being recorded (`keybindings_tab::recording`). No key
/// reaches a shortcut then. With a list open, Escape closed the list and
/// also left edit mode.
/// Whether a shortcut is being recorded (`keybindings_tab::recording`):
/// a global shortcut pressed meanwhile is the recorder's, not its action.
pub fn recording_shortcut(ctx: &egui::Context) -> bool {
    keybindings_tab::recording(ctx)
}

/// Hand a recording the chord a global shortcut's grab took
/// (`keybindings_tab::record_grabbed`).
pub fn record_grabbed_chord(ctx: &egui::Context, chord: crate::keybindings::KeyChord) {
    keybindings_tab::record_grabbed(ctx, chord);
}

pub fn keyboard_held(ctx: &egui::Context) -> bool {
    command_palette_open(ctx)
        || ctx.memory(|m| m.any_popup_open())
        || keybindings_tab::recording(ctx)
}

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
    /// Keep for a paste (`crate::clipboard`).
    Copy(usize),
    /// Keep for a paste, and take out of the scene.
    Cut(usize),
    ResetTransform(usize),
    ToggleGravity(usize),
    BringForward(usize),
    SendBackward(usize),
    Group(usize),
    Ungroup(usize),
    Arrange(usize, crate::input::arrange::Arrange),
    /// Save its animation as a GIF (`crate::gif_export`); the loops ask
    /// where, as they do for sharing a scene.
    SaveGif(usize),
    /// Share it as one file (`crate::scene_file`, `.animachar`); asked
    /// where likewise.
    ShareCharacter(usize),
}

/// What the Scene tab asks of the caller, which has what it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneRequest {
    /// "Add file…": open the file chooser.
    AddFile,
    /// "Paste": add what was copied (`crate::clipboard`).
    Paste,
    /// Share the saved scene in this file as one file
    /// (`crate::outcomes::SceneTransfer`).
    Share(std::path::PathBuf),
    /// "Import…": open a shared scene.
    Import,
    /// Appearance's "Back up…": everything in one file
    /// (`scene_file::backup`).
    BackUp,
    /// Appearance's "Restore…": a backup, over these settings.
    Restore,
}

/// The settings for holding still while nobody is there (`crate::away`),
/// and whether this session can tell.
pub struct AwayControls<'a> {
    pub idle_minutes: &'a mut u32,
    pub on_battery: &'a mut bool,
    /// Greeted back after being away (`crate::welcome`).
    pub welcome_back: &'a mut bool,
    /// Whether the user's idle time can be read here; `None` until the
    /// watch has looked.
    pub idle_available: Option<bool>,
}

/// A collapsing section with the design-system heading, its open flag
/// kept in [`CollapseState`] so it survives a restart: seeded from `open`
/// on the first frame, and written back from the widget's animated
/// openness so a click marks the config dirty exactly once, not on every
/// frame of the animation. `id` is stable — not `title`, which follows
/// the language and may carry a count.
pub(super) fn section(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    open: &mut bool,
    config_dirty: &mut bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let header = egui::RichText::new(title).text_style(theme::h2());
    let response = egui::CollapsingHeader::new(header)
        .id_salt(("anima.section", id))
        .default_open(*open)
        .show(ui, |ui| {
            ui.add_space(theme::SPACE_XS);
            add_contents(ui);
        });
    let visually_open = response.openness > 0.5;
    if visually_open != *open {
        *open = visually_open;
        *config_dirty = true;
    }
    ui.add_space(theme::SPACE_S);
}

/// The sounds' controls in Appearance (`crate::audio`).
pub struct SoundControls<'a> {
    /// `sound_volume`, 0 to 1.
    pub volume: &'a mut f32,
    pub muted: &'a mut bool,
    pub quiet_at_bedtime: &'a mut bool,
}

/// What the right-click menu offers beyond its fixed entries, worked out
/// before the frame from what it acts on (`crate::outcomes::menu_offers`).
#[derive(Debug, Clone, Copy, Default)]
pub struct MenuOffers {
    /// How many characters it acts on; two or more can be arranged.
    pub count: usize,
    /// They can be made a group: two or more, not already exactly one.
    pub group: bool,
    /// At least one of them is in a group.
    pub ungroup: bool,
}

/// What `context_menu` decided about its own state for this frame.
pub enum ContextMenuOutcome {
    /// Menu remains visible — nothing happened this frame. `settled`
    /// is true once no pointer button is held: from then on, and not
    /// before, a click outside the menu may dismiss it (see
    /// `ContextMenuState::armed`).
    Open { settled: bool },
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

/// Scroll the panel to a control that has just got the keyboard's focus.
/// egui moves the focus with Tab but does not scroll to it, so tabbing
/// down a long tab went on past the panel's edge, the focused control out
/// of sight — in the Inspector, the rows for speaking, sounds and bubbles.
/// Called at the end of the scrolled content, where this pass has laid out
/// the control it may name; a control outside that content (the tab bar,
/// a popup) or already in view is left alone.
fn keep_focus_in_view(ui: &mut egui::Ui) {
    let seen_id = egui::Id::new("anima.settings.focus-seen");
    let focused = ui.memory(|m| m.focused());
    let seen = ui
        .data(|d| d.get_temp::<Option<egui::Id>>(seen_id))
        .flatten();
    if focused == seen {
        return;
    }
    ui.data_mut(|d| d.insert_temp(seen_id, focused));
    let Some(rect) = focused
        .and_then(|id| ui.ctx().read_response(id))
        .map(|response| response.rect)
    else {
        return;
    };
    if ui.min_rect().contains_rect(rect) && !ui.clip_rect().contains_rect(rect) {
        ui.scroll_to_rect(rect, None);
    }
}

/// A tab a shortcut asked for, shown at the panel's next frame: 0 for
/// none, else one more than its place in [`SettingsTab::ALL`].
static ASKED_TAB: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Show the Inspector, with what is selected (`Action::ShowEntityInfo`).
pub fn show_inspector() {
    ask_for_tab(SettingsTab::Inspector);
}

/// Show the Keybindings tab, every shortcut as it is bound
/// (`Action::ShowHelp`).
pub fn show_keybindings() {
    ask_for_tab(SettingsTab::Keybindings);
}

fn ask_for_tab(tab: SettingsTab) {
    let place = SettingsTab::ALL.iter().position(|t| *t == tab).unwrap_or(0);
    ASKED_TAB.store(place as u8 + 1, std::sync::atomic::Ordering::Relaxed);
}

fn take_asked_tab() -> Option<SettingsTab> {
    match ASKED_TAB.swap(0, std::sync::atomic::Ordering::Relaxed) {
        0 => None,
        n => SettingsTab::ALL.get(usize::from(n) - 1).copied(),
    }
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
#[expect(clippy::too_many_arguments)]
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
    window_awareness_supported: bool,
    snap_while_dragging: &mut bool,
    characters_bump: &mut bool,
    characters_greet: &mut bool,
    active_scene: &mut Option<String>,
    reminders: &mut Vec<crate::reminders::ReminderConfig>,
    scene_schedule: &mut Vec<crate::schedule::ScheduleRule>,
    focus: &mut crate::focus::FocusConfig,
    reminder_sound: &mut crate::reminders::ReminderSound,
    notify_reminders: &mut bool,
    // Whether one overlay can cover every monitor. False on native
    // Wayland, where a layer surface belongs to a single output.
    span_supported: bool,
    reduced_motion: &mut bool,
    large_bubbles: &mut bool,
    hover_startle: &mut bool,
    on_fullscreen: &mut crate::fullscreen::OnFullscreen,
    step_aside_apps: &mut Vec<String>,
    // Above other windows, or under them on the desktop.
    always_on_top: &mut bool,
    tap_characters: &mut bool,
    little_effects: &mut bool,
    bedtime: &mut crate::bedtime::Bedtime,
    moods: &mut bool,
    away: AwayControls<'_>,
    sounds: SoundControls<'_>,
    start_at_login: &mut bool,
    monitors: &[MonitorInfo],
    library: Option<&LibraryIndex>,
    library_outcome: &mut Option<LibraryOutcome>,
    keybindings: &mut KeyBindings,
    collapse_state: &mut CollapseState,
    accesskit_enabled: &mut bool,
    // Whether this backend has a screen-reader bridge. egui's comes with
    // egui-winit; the native Wayland loop has none.
    accesskit_supported: bool,
    warnings: &std::collections::BTreeSet<Warning>,
    last_seen_whats_new: &mut Option<String>,
    hotkey_backend: crate::hotkeys::probe::HotkeyStatus,
    shimeji_import: &mut Option<String>,
    // Set when "Add file…" or "Paste" is clicked; the caller acts on it.
    scene_request: &mut Option<SceneRequest>,
) {
    // Both backends run this every frame, so this is where an input
    // method composing Chinese, Japanese or Korean gets the face it needs.
    crate::ui::icons::load_cjk_for_ime(ctx);
    // And the face the language itself needs, should it have changed
    // without the Appearance picker — a reload of the config file, a
    // restored backup — which re-installs the fonts on its own.
    if crate::ui::icons::locale_needs_cjk(&crate::i18n::current_locale()) {
        crate::ui::icons::install_with_cjk(ctx);
    }

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
        .stroke(egui::Stroke::new(1.0_f32, palette.border_subtle));
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
            if let Some(asked) = take_asked_tab() {
                active_tab = asked;
            }
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
            // condition clears (`config_watch` takes them back).
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
            //
            // Only on the default tab. It used to render above *every*
            // tab body, so the same four-item changelog reappeared each
            // time the user switched tab, competing with the content
            // they had just navigated to. The active tab is `insert_temp`
            // (in-process only), so every session opens on Inspector and
            // an upgrader still meets the panel exactly once.
            if active_tab == SettingsTab::Inspector
                && crate::ui::whats_new::show(ui, last_seen_whats_new)
            {
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
            // Up to the footer below, not under it: it used to cover the
            // last row of a long tab, which could not be scrolled to.
            let footer =
                ui.text_style_height(&theme::caption()) + SPACE_S + ui.spacing().item_spacing.y;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height((ui.available_height() - footer).max(0.0))
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
                                window_awareness_supported,
                                snap_while_dragging,
                                characters_bump,
                                characters_greet,
                                active_scene,
                                reminders,
                                scene_schedule,
                                focus,
                                reminder_sound,
                                notify_reminders,
                                span_supported,
                                monitors,
                                collapse_state,
                                scene_request,
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
                                accesskit_supported,
                                reduced_motion,
                                large_bubbles,
                                hover_startle,
                                on_fullscreen,
                                step_aside_apps,
                                always_on_top,
                                tap_characters,
                                little_effects,
                                away,
                                sounds,
                                bedtime,
                                moods,
                                start_at_login,
                                scene_request,
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
                    keep_focus_in_view(ui);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tab_asked_for_is_shown_once() {
        show_keybindings();
        assert_eq!(take_asked_tab(), Some(SettingsTab::Keybindings));
        assert_eq!(take_asked_tab(), None, "taken once");
        show_inspector();
        assert_eq!(take_asked_tab(), Some(SettingsTab::Inspector));
    }
}
