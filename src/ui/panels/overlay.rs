//! The overlay's egui, one pass of it, the same on both paths: the ⚙, the
//! effects and the speech bubbles, the first-run tour, the settings panel,
//! in edit mode the right-click menu and the command palette, then the
//! toasts and the perf overlay. The winit path calls it from its
//! `ui.render` closure, the native Wayland loop from its renderer's; what
//! differs between them comes in an [`Overlay`].

use super::{
    command_palette, context_menu, settings, toasts, toggle_button, ContextMenuOutcome,
    LibraryOutcome, MenuOffers, PaletteOutcome, SceneRequest, Supports,
};
use crate::asset_library::LibraryIndex;
use crate::config::AppConfig;
use crate::input::selection::SelectionState;
use crate::monitor::MonitorInfo;
use crate::scene::Scene;
use crate::ui::banner::Warning;
use crate::ui::ToastQueue;

/// What the overlay's egui draws from in a frame ([`overlay`]).
pub(crate) struct Overlay<'a> {
    /// Edit mode: the panel open, the right-click menu and the palette.
    pub edit_mode: bool,
    /// The ⚙ outside edit mode (`GlobalConfig::show_toggle_button`).
    pub show_toggle_button: bool,
    /// Nothing at all: the overlay is hidden and still mapped, as a layer
    /// surface stays (native Wayland). The winit path unmaps its window.
    pub hidden: bool,
    /// The effects' and bubbles' positions are in the window's pixels
    /// (winit), not already in points (native Wayland).
    pub in_pixels: bool,
    pub effects: &'a [crate::effects::Drawn],
    pub bubbles: &'a [crate::speech::Shown],
    pub toasts: &'a ToastQueue,
    pub scene: &'a mut Scene,
    pub selection: &'a mut SelectionState,
    pub config_dirty: &'a mut bool,
    pub config: &'a mut AppConfig,
    pub supports: Supports,
    pub monitors: &'a [MonitorInfo],
    pub library: Option<&'a LibraryIndex>,
    pub warnings: &'a std::collections::BTreeSet<Warning>,
    pub hotkey_backend: crate::hotkeys::probe::HotkeyStatus,
    /// The right-click menu when it is on this overlay, and what it offers.
    pub menu: Option<&'a crate::app::ContextMenuState>,
    pub menu_offers: MenuOffers,
    /// The perf overlay, while it shows.
    pub perf: Option<PerfShown<'a>>,
}

/// What the perf overlay shows ([`Overlay::perf`]).
pub(crate) struct PerfShown<'a> {
    pub sampler: &'a crate::perf::PerfSampler,
    pub rss_kib: Option<u64>,
    pub gpu: crate::ui::perf_overlay::GpuStats,
}

/// What the overlay's egui asked for in a frame, set as its passes go:
/// applied after the frame, where the caller is free to.
#[derive(Default)]
pub(crate) struct OverlayAsked {
    /// The ⚙ pressed: edit mode on or off.
    pub toggle: bool,
    /// The right-click menu: open, closed, or an action picked.
    pub menu: Option<ContextMenuOutcome>,
    /// The command palette's pick.
    pub palette: Option<PaletteOutcome>,
    /// The library's: an asset to add.
    pub library: Option<LibraryOutcome>,
    /// A Shimeji pack to import, by path.
    pub shimeji_import: Option<String>,
    /// A scene to paste, share, import, back up or restore, or a file to add.
    pub scene_request: Option<SceneRequest>,
    /// "Export" in the perf overlay.
    pub perf_export: bool,
}

/// One egui pass of the overlay ([`Overlay`]), what it asks for in `asked`.
pub(crate) fn overlay(ctx: &egui::Context, o: Overlay<'_>, asked: &mut OverlayAsked) {
    let Overlay {
        edit_mode,
        show_toggle_button,
        hidden,
        in_pixels,
        effects,
        bubbles,
        toasts: toast_queue,
        scene,
        selection,
        config_dirty,
        config,
        supports,
        monitors,
        library,
        warnings,
        hotkey_backend,
        menu,
        menu_offers,
        perf,
    } = o;
    crate::ui::motion::set_reduced(ctx, config.global.reduced_motion);
    if hidden {
        // Hidden means hidden: no sprites (the caller draws none) and no
        // UI either, so the ⚙ button doesn't linger over a supposedly
        // hidden overlay. Restore comes through the same D-Bus/keybinding
        // channel that hid it.
        return;
    }
    // The toggle button: the one control in pass-through; in edit mode it
    // sits on top of everything else.
    if (edit_mode || show_toggle_button) && toggle_button(ctx, edit_mode) {
        asked.toggle = true;
    }
    let scale = if in_pixels {
        ctx.pixels_per_point()
    } else {
        1.0
    };
    crate::ui::effects::paint(ctx, effects, scale);
    crate::ui::speech::paint(ctx, bubbles, scale);
    // And to a screen reader (`crate::announce`), with the toasts.
    crate::ui::speech::announce(ctx);
    crate::ui::toasts::announce(ctx, toast_queue);
    // First-run tour (V.2): floats on the overlay in both modes; advances
    // to its interactive steps in edit mode.
    if crate::ui::onboarding::coach_marks(
        ctx,
        &mut config.global.onboarding,
        edit_mode,
        show_toggle_button,
        &config.keybindings,
    ) {
        *config_dirty = true;
    }
    // Unconditional call: `open` drives SidePanel::show_animated, so
    // leaving edit mode slides the panel out instead of snapping it away.
    settings(
        ctx,
        edit_mode,
        scene,
        selection,
        config_dirty,
        config,
        supports,
        monitors,
        library,
        &mut asked.library,
        warnings,
        hotkey_backend,
        &mut asked.shimeji_import,
        &mut asked.scene_request,
    );
    if edit_mode {
        if let Some(state) = menu {
            asked.menu = Some(context_menu(ctx, state, menu_offers));
        }
        // Ctrl+K opens the command palette.
        asked.palette = command_palette(
            ctx,
            &config.keybindings,
            selection.selected_index().is_some(),
        );
    }
    // In both modes: one raised outside edit mode — a reload, a paused
    // scene — used to be gone before the panel opened.
    toasts(ctx, toast_queue);
    // Above every panel, so someone chasing a stutter doesn't have to hunt
    // for it behind one.
    if let Some(perf) = perf {
        if crate::ui::perf_overlay::show(ctx, perf.sampler, perf.rss_kib, perf.gpu).is_some() {
            asked.perf_export = true;
        }
    }
}
