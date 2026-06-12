# UX audit — 0.8 (V.0)

Method: code-walk of every UI surface (panels, toasts, overlays,
context menu, palette) against four heuristics — i18n completeness,
theme/contrast discipline, keyboard reachability, design-system
spacing — plus a manual visual matrix (§Manual pass) for what static
analysis can't see. Per the plan, the `fix` list below **is** the
V.1–V.6 backlog; anything discovered later goes to 0.9-bugfix.

Severity: **blocker** (breaks a core promise — e.g. localized UI),
**paper-cut** (visible roughness), **nice** (polish beyond contract).

## Findings

### i18n completeness

| # | Surface | Finding | Severity | Verdict |
|---|---------|---------|----------|---------|
| F1 | Toasts (app layer) | ~15 hardcoded English toast strings bypass `i18n::t`: "Config saved", "Save failed: …", "Rejected: …", "Added …", "Load failed: …", "Theme: …", "Couldn't add preset entry", "Duplicated …", "Duplicate failed", "Playback resumed/paused" (src/app/mod.rs, input.rs, outcomes.rs), "Asset library not wired…" (wayland/run.rs). A ro/ja/de session shows mixed-language toasts. | blocker | fix (V.6) |
| F2 | Context menu | Every item hardcoded: "Duplicate", "Reset transform", "Toggle gravity", "Bring forward", "Send backward", + the rest of context_menu.rs. Primary right-click surface. | blocker | fix (V.6) |
| F3 | Command palette | "Replace" / "Append" buttons and the "Esc to close · Ctrl+K to toggle" footer hint are hardcoded. | blocker | fix (V.6) |
| F4 | Inspector | `"z-index"`, `"Wander box"` labels hardcoded. (`"X"`/`"Y"` axis labels: universal symbols, exempt.) | paper-cut | fix (V.6); X/Y wontfix (axis symbols) |
| F12 | Inspector sliders | Every `Slider::.text(...)` label was a hardcoded English literal — and the **translated keys already existed in all 10 locales** (prepared in the D-phase locale audit, never wired into the code). Missed by the original `ui.label/button` grep; found while implementing V.3. Wired + units added. | blocker | fix (V.3, done) |
| F5 | Perf overlay | "FPS", "RSS", row labels are raw English. Developer-facing HUD with fixed-width number alignment; translating would break the monospace layout for zero user value. | nice | wontfix (dev surface) |

### Theme & contrast

| # | Surface | Finding | Severity | Verdict |
|---|---------|---------|----------|---------|
| F6 | Toggle button | Colors inline: active green `(40,160,60)`, dim `(50,50,60,200)`, `WHITE` glyph — invisible to the HC theme variants; white-on-green is ~2.1:1 at small sizes. Move to theme palette so HC overrides apply. | paper-cut | fix (V.5) |
| F7 | Keybindings tab | Inline amber `(220,180,60)` and blue `(100,180,220)` accents bypass the theme; unverified against HC AAA. | paper-cut | fix (V.5) |

### Keyboard & a11y

| # | Surface | Finding | Severity | Verdict |
|---|---------|---------|----------|---------|
| F8 | All panels | One `request_focus` call in the whole UI tree — tab order is egui-default everywhere; no visible focus styling on custom widgets (toggle button, palette rows, library grid). Keyboard-only walkthrough almost certainly dead-ends at the context menu (mouse-only by nature) and the library grid. | blocker | fix (V.4) |
| F9 | Motion | No reduced-motion knob; panel/toast/palette animations are unconditional. Planned. | paper-cut | fix (V.1) |

### Design-system discipline

| # | Surface | Finding | Severity | Verdict |
|---|---------|---------|----------|---------|
| F10 | Panels | One raw `add_space(N)` literal outside the `SPACE_*` scale; everything else conforms. | nice | fix (V.5) |
| F11 | Inspector | Sliders lack units in-label (px/%/fps appear inconsistently); no double-click-to-reset. Planned. | paper-cut | fix (V.3) |

### Healthy (verified, no action)

- Tooltips: every `on_hover_text` routes through `t()` (0 literals).
- Spacing scale: 1 violation total (F10).
- Empty states: all use the `states::` helpers with `t()` keys.
- New-string discipline held through 0.6/0.7 for *panel* strings —
  the leaks concentrate in the app layer (toasts) and the two
  pre-i18n surfaces (context menu, palette buttons).

## Manual pass (visual matrix) — PENDING

Static analysis can't judge rendered contrast, clipping, or floating
layout. Walk {Dark, Light, Dark-HC, Light-HC} × {en, ro} ×
{mouse, keyboard-only} over: settings panel (all 5 tabs), context
menu, command palette, toasts, perf overlay, onboarding coach-marks,
empty states, the Scene-tab window-awareness toggle (new, scope
exception). Record findings here with the F-numbering continued.
This pass needs a human eye + a real session; schedule alongside
V.4's Orca walkthrough.

## Backlog mapping

- **V.1** ← F9 (reduced motion) + planned transitions
- **V.3** ← F11 (units, double-click-reset, grouping) + F12 (slider labels)
- **V.4** ← F8 (focus order, visible focus, Orca)
- **V.5** ← F6, F7, F10 (theme palette extraction, spacing)
- **V.6** ← F1–F4 (i18n leak closure + native review of all 0.6–0.8 strings)
