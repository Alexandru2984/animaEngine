# Design system

Single source of truth for **colors, spacing, typography, radii, icons**
across animaEngine's egui UI. Sub-phases A.1+ implement what's specified
here; if you find yourself reaching for a hardcoded value in a panel,
add it to this doc and reference the constant instead.

The goal isn't novelty — it's coherence. Every surface should feel like
the same product, not the union of someone's mood swings across three
weeks of development.

---

## 1. Color system

Two themes for 0.2.0: **Dark** (default) and **Light**. The architecture
allows more themes later (high-contrast, user-defined) without
restructuring.

Each theme defines the same 16-token table. Sub-phase A.1 implements
this as `Theme::Dark.palette()` and `Theme::Light.palette()` returning
a `Palette` struct.

### 1.1 Background tiers

Three levels of "what's behind something." Helps the eye separate
overlapping surfaces.

| Token | Dark | Light | Used for |
|-------|------|-------|----------|
| `bg.base` | `#15181E` | `#FBFBFC` | Window background (transparent overlay rarely shows this; it's the egui canvas color when not transparent) |
| `bg.surface` | `#1E222B` | `#F2F3F6` | Panels, settings sidebar |
| `bg.elevated` | `#262B36` | `#FFFFFF` | Popups, context menus, toasts, tooltips |

### 1.2 Foreground (text) tiers

| Token | Dark | Light | Used for |
|-------|------|-------|----------|
| `fg.primary` | `#E8EAEF` | `#1A1D23` | Headlines, default body |
| `fg.secondary` | `#A8ADB7` | `#5A6070` | Sub-labels, captions |
| `fg.muted` | `#6B7280` | `#9CA3AF` | Disabled, placeholder, weak text |
| `fg.inverse` | `#15181E` | `#FBFBFC` | Text on filled accent buttons |

### 1.3 Accent

The brand color — purple-blue. Two stops so we can do
hover/active/pressed states without ad-hoc lighten/darken.

| Token | Dark | Light | Used for |
|-------|------|-------|----------|
| `accent.base` | `#7C8EFF` | `#5163E8` | Selected items, primary buttons, focus rings, key labels |
| `accent.hover` | `#9AA8FF` | `#6C7DF0` | :hover on accent surfaces |
| `accent.subtle` | `#7C8EFF22` | `#5163E81F` | Background tint behind selected list rows |

### 1.4 Semantic

For toast notifications, status badges, validation states.

| Token | Dark | Light |
|-------|------|-------|
| `semantic.success` | `#5BCB7B` | `#1F9E55` |
| `semantic.warn` | `#E8B23C` | `#B07900` |
| `semantic.error` | `#F26565` | `#C7322F` |
| `semantic.info` | `#5FA8E8` | `#2C70C2` |

Each has a `.subtle` variant at 13–16 % alpha for filled-background
toasts/badges where text on top must stay readable.

### 1.5 Borders & dividers

| Token | Dark | Light | Used for |
|-------|------|-------|----------|
| `border.subtle` | `#FFFFFF0F` | `#0000000F` | Panel separators, list row dividers |
| `border.strong` | `#FFFFFF22` | `#0000001F` | Outlines on enabled buttons, slider tracks |
| `border.focus` | `accent.base` | `accent.base` | 2 px ring on the keyboard-focused widget |

### 1.6 Shadow / elevation

Three tiers; rendered via egui Shadow.

| Token | Spec (offset, blur, alpha) | Used for |
|-------|---------------------------|----------|
| `elev.low` | `(0, 1px, 0.18)` | Buttons on hover, settings panel edge |
| `elev.mid` | `(0, 4px, 0.22)` | Toasts, popup menus |
| `elev.high` | `(0, 8px, 0.28)` | Modal-style cards (onboarding, presets dialog) |

---

## 2. Typography

We don't ship our own font — egui's default (`Hack` / `Inter` mix
depending on locale) is the baseline. We define **a scale** so every
text use site picks one of the named sizes.

| Token | Size (logical px) | Weight | Use |
|-------|---------------------|--------|-----|
| `text.h1` | 22 | 700 | Settings panel title, dialog headers |
| `text.h2` | 17 | 600 | Section headers in tabs |
| `text.body` | 13.5 | 400 | Default for labels, descriptions |
| `text.body-strong` | 13.5 | 600 | Field names in inspectors |
| `text.caption` | 11.5 | 400 | Helper text below controls |
| `text.code` | 12 | 400 (mono) | Asset paths, IDs, hex values |

Line height is fixed at 1.45× the size. Letter spacing only on `h1` /
`h2` (−0.5 % to tighten heavy weights).

---

## 3. Spacing

Single 4-px base unit. All paddings, margins, and gaps round to one of
these tokens — no `padding: 7px` sins.

| Token | Value | Use |
|-------|-------|-----|
| `space.xs` | 2 | Icon-to-text inside a chip |
| `space.s`  | 4 | Tight inline gap |
| `space.m`  | 8 | Default control padding, gap between siblings |
| `space.l`  | 12 | Between sections in a panel |
| `space.xl` | 16 | Panel outer padding, dialog padding |
| `space.2xl`| 24 | Onboarding card padding, between groups |
| `space.3xl`| 32 | Page-level separation in tall panels |

---

## 4. Radius

Three tiers, applied via egui's `rounding`.

| Token | Value | Use |
|-------|-------|-----|
| `radius.sm` | 4 | Pills, badges, checkboxes, slider thumbs |
| `radius.md` | 6 | Buttons, text inputs, dropdowns |
| `radius.lg` | 12 | Cards, dialogs, toast frames |

The toggle ⚙ button keeps `radius.sm` to feel like a corner widget,
not a card.

---

## 5. Iconography

### Source

We use the **Phosphor icon font** via the `egui-phosphor` crate
(font-based, 6000+ icons, no system deps, MIT). Phosphor's regular
weight is our baseline — friendly without being childish.

### Sizing & alignment

- **Default icon size**: 16 px (matches `text.body` x-height; sits
  cleanly inline with labels)
- **Tab/header icons**: 18 px
- **Toolbar / hero icons** (e.g. the ⚙ button): 24 px
- **Toast severity icons**: 18 px, leftmost

### Pairing with text

When an icon precedes text in a button or menu item:

- Use `space.s` (4 px) between icon and text — closer than `space.m`
  because they belong to the same word group.
- Vertical alignment: icon baseline matches text baseline.

### Coloring

Icons inherit the surrounding text color by default (`fg.primary` /
`fg.secondary` / etc.). Exceptions:

- **Severity icons** in toasts use the matching `semantic.*` token.
- **Destructive actions** (delete, remove) tint icon with
  `semantic.error` even when text stays primary.

### Phosphor naming convention

When referenced in code, use the Phosphor name (e.g. `gear-six`, not
"settings"). We document the chosen icon next to each UI string in
`src/ui/icons.rs` (A.2 deliverable).

---

## 6. Motion

Used sparingly — animation that doesn't serve a purpose feels cheap.

| Use | Duration | Easing |
|-----|----------|--------|
| Toast slide-in | 200 ms | `ease-out-quad` |
| Toast fade-out | 300 ms | `ease-in-quad` |
| Context menu fade + scale | 120 ms | `ease-out-quad` |
| Hover transitions (background, border) | 90 ms | `ease-out-quad` |
| Tab content cross-fade | 100 ms | linear |
| Selection pulse | 2 s cycle | sine, low amplitude (0.0 → 0.2 alpha) |
| Focus ring appear | 80 ms | linear |

Anything longer than 300 ms in a UI transition is a bug; we'd rather
feel snappy than smooth.

`ease-out-quad` = `1.0 - (1.0 - t).powi(2)`. egui's `lerp` plus a frame
delta is enough — we don't pull in a tween crate.

---

## 7. Component patterns

How the tokens combine for the recurring UI building blocks.

### 7.1 Button (default)

- Background: `bg.elevated`
- Text: `fg.primary` (`text.body-strong`)
- Border: `border.strong`
- Radius: `radius.md`
- Padding: `space.m` × `space.s` (8 × 4 px)
- Hover: bg → mix(`bg.elevated`, `accent.base`, 8 %), border →
  `border.focus`
- Active (pressed): bg → mix(`bg.elevated`, `accent.base`, 16 %)
- Disabled: bg unchanged, text → `fg.muted`

### 7.2 Button (primary)

- Background: `accent.base`
- Text: `fg.inverse` (`text.body-strong`)
- Border: none
- Hover: bg → `accent.hover`

### 7.3 Button (destructive)

- Background: `bg.elevated`
- Text + leading icon: `semantic.error`
- Border: `border.strong`
- Hover: border → `semantic.error`, bg → `semantic.error` at 8 % alpha

### 7.4 Slider

- Track: `border.strong`, height 4 px, `radius.sm`
- Active portion: `accent.base`
- Thumb: 14 × 14 circle, `accent.base` on `bg.elevated` border
- Label above with `text.caption` showing the current value

### 7.5 Text input

- Background: `bg.elevated`
- Text: `fg.primary` (`text.body`)
- Border: `border.subtle` default, `border.focus` 2 px on focus
- Placeholder: `fg.muted`
- Padding: `space.m` × `space.s`

### 7.6 Checkbox / toggle

- Off: 16 × 16 square, border `border.strong`, bg `bg.elevated`,
  `radius.sm`
- On: bg `accent.base`, checkmark `fg.inverse`, no border
- Label uses `text.body`; gap `space.m`

### 7.7 List row (e.g. scene entities)

- Default: transparent bg, text `fg.primary`
- Hover: bg `accent.subtle`
- Selected: bg `accent.subtle`, left border 2 px `accent.base`,
  `text.body-strong`
- Padding: `space.m` × `space.s`

### 7.8 Toast

- Background: `bg.elevated`
- Left accent stripe: 3 px wide, `semantic.*` matching severity
- Leading severity icon: 18 px, `semantic.*`
- Text: `fg.primary` (`text.body`)
- Optional action button: text button styled `accent.base`,
  `text.caption`
- Radius: `radius.lg`
- Padding: `space.l` × `space.m`
- Shadow: `elev.mid`
- Stack gap: `space.s`

### 7.9 Toggle button ⚙ (corner widget)

- Pass-through mode: `bg.elevated` background, `fg.secondary` icon,
  `border.subtle`
- Edit mode: `accent.base` background, `fg.inverse` icon, no border
- Hover: scale 1.05, `elev.low` shadow

### 7.10 Dialog / modal card

- Background: `bg.elevated`
- Border: `border.subtle`
- Radius: `radius.lg`
- Padding: `space.2xl` all around
- Shadow: `elev.high`
- Backdrop: `bg.base` at 60 % alpha covering everything else

---

## 8. Empty / error / loading states

Patterns rather than tokens — covered in sub-phase A.4 implementation
but defined here so they stay consistent.

- **Empty state card**: centered in panel, `radius.lg`, padding
  `space.2xl`. Icon 32 px in `fg.muted`, headline `text.h2`,
  one-line helper in `text.body` `fg.secondary`, and 1–3 action
  chips below.
- **Loading**: 3-dot spinner using `accent.base` with staggered
  alpha; never a full-screen overlay — local to the panel that's
  waiting.
- **Error**: same shape as empty state but icon in `semantic.error`,
  headline `text.h2`, with a primary action button at the bottom
  ("Retry" / "Open settings" / "View logs").

---

## 9. Accessibility commitments

- All interactive widgets have visible focus rings using
  `border.focus` (handled by egui via `set_visuals`).
- Color is never the sole signal for state — selection adds a left
  border, severity adds an icon, etc.
- Minimum contrast: text on background ≥ 4.5:1 (WCAG AA). The dark and
  light palettes were eyeballed against this; the A.9 deliverable
  includes a contrast audit script that fails CI on regressions.
- High-contrast variants of both themes ship as `Theme::DarkHC` /
  `Theme::LightHC` toggle in App settings (A.9).

---

## 10. Implementation map

How this doc lands in code, sub-phase by sub-phase:

| Doc section | Lands in | Sub-phase |
|-------------|----------|-----------|
| §1 Color system | `src/ui/theme.rs::Palette` + `Visuals` apply | A.1 |
| §2 Typography | `Style::text_styles` map | A.1 |
| §3 Spacing | `pub const SPACING_*` in `theme.rs` | A.1 |
| §4 Radius | `pub const RADIUS_*` | A.1 |
| §5 Iconography | `src/ui/icons.rs` re-exports + names | A.2 |
| §6 Motion | `src/ui/anim.rs` helpers (lerp + ease curves) | A.5 |
| §7 Component patterns | `src/ui/widgets.rs` helpers wrapping egui primitives | A.1 + A.3 |
| §8 Empty/error/loading | `src/ui/states.rs` | A.4 |
| §9 Accessibility | high-contrast palettes + contrast test | A.9 |

After A.11 this document is the canonical reference. Adding a new
color or spacing value means editing this file first, then the code.
