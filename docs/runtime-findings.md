# Runtime findings — interactive / visual defects

Defects found by **running the application and looking at it**, rather than by
static analysis, fuzzing or unit tests. Every entry here was observed on
screen; none of them would have been caught by the test suite, because the
test suite covers the parsers and the pure logic, not the rendered product.

Status legend: `OPEN` needs fixing · `FIXED` resolved, kept for history ·
`BY DESIGN` observed, deliberate, not changing · `RETRACTED` reported here
in error, kept so the mistake isn't repeated.

**Current state: nothing `OPEN`.** R1–R5, R7–R18 are `FIXED`, R6 is
`BY DESIGN`, R6b is `RETRACTED`. The unexplored surfaces listed at the
bottom are where the next round should start.

## How these were reproduced

A headless [sway](https://swaywm.org) session plus a virtual pointer, so the
app can be driven and screenshotted without touching a real desktop:

```sh
# 1. Compositor with a virtual output
WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 \
WLR_RENDERER=pixman sway -c <minimal config>

# 2. The app, with its XDG dirs sandboxed so a test never touches real config
XDG_CONFIG_HOME=... XDG_CACHE_HOME=... XDG_DATA_HOME=... \
ANIMA_USE_WAYLAND_NATIVE=1 ./target/debug/anima_engine

# 3. Screenshot
grim out.png
```

Two things make this harder than it looks, and are worth writing down:

- **`wlrctl` does not work here.** It creates its virtual pointer, sends one
  action and exits, so the seat gains and loses its pointer capability before
  a client can bind a `wl_pointer` — nothing is ever delivered. Input needs a
  pointer that stays alive for the whole script, and that pointer must exist
  *before* the app starts, or the app has no pointer to bind at startup.
- **`sway`'s `seat cursor` IPC commands report success but deliver nothing**
  when the seat has no pointer device, for the same reason.

Absolute positioning (`zwlr_virtual_pointer_v1::motion_absolute`) is required
to aim at a widget; `wlrctl`'s motion is relative only.

---

## Interaction

### R1 · Context menu closes on the release of the click that opened it — `FIXED`

Right-clicking an entity opens the context menu on button *press* and closes
it again on *release*, so a normal right-click makes it flash and vanish. The
menu is only usable while the button is physically held down.

Verified by holding the right button: the full menu renders correctly
(Duplicate / Reset transform / Toggle gravity / Bring forward / Send backward
/ Delete). On release it is gone.

This makes the context menu — six of the app's entity actions — effectively
unreachable in normal use.

Observed on the native Wayland path. **Not** verified on the winit/X11 path,
which sets the menu state from the same `Pressed` event and may share the
defect.

**Fixed.** Two compounding causes: the menu was anchored with its top-left
exactly at the click point, so the pointer sat on the rect boundary where
egui reports `contains_pointer() == false` (probed: `any_click=true`,
`contains_pointer=false`, `interact_pos == rect.min`); and dismissal did not
exclude the click that opened the menu. The menu is now offset 2px off the
cursor, and `ContextMenuState` carries an `armed` flag set once it has been
shown. The shared fix covers both backends. Verified in both directions:
a normal right-click leaves the menu up, a later click outside dismisses it.

### R2 · No entity drag on the native Wayland path — `FIXED`

`DragController` is never used anywhere under `src/wayland/`. Dragging a
character with the mouse — arguably the core interaction of a desktop-pet
application — is not implemented on the native backend at all.

Worse, `src/wayland/run.rs`'s own module doc has a "What doesn't (yet)"
section listing three limitations (FollowCursor in pass-through,
window-awareness physics, `XGrabKey` hotkeys) and **drag is not among them**,
so the documentation implies it works.

**Fixed.** `App::handle_mouse_input`'s behaviour is now mirrored on the
Wayland path — press selects and picks up, motion moves (invalidating the
Bounce rest position), release drops, and a press/release that never moved
pokes instead. The module doc now lists this under "What works". Verified by
dragging a character across a headless sway session.

### R3 · Left-click does not select an entity on Wayland — `FIXED`

`src/wayland/run.rs` only matches `egui::PointerButton::Secondary` when
resolving a click to an entity, so selection is right-click only. The winit
path selects on left-click (`src/app/input.rs`). The Inspector's own empty
state says *"Click an entity in the Scene tab, or press Tab to cycle"* — it
does not mention that clicking the sprite works differently per backend.

**Fixed** as part of R2 — left press now selects, and clicking empty space
deselects, matching winit.

---

## Content

### R4 · A 1.0.0 release greets every new user with "What's new in 0.4" — `FIXED`

`WHATS_NEW_VERSION` in `src/ui/whats_new.rs` is `"0.4.0"`, and the header
string is translated as "0.4" in **all ten locales**:

```
src/i18n/locales/en.ftl:192:whats-new-header = What's new in 0.4
… de, es, fr, it, ja, nl, pl, pt-BR, ro — all the same
```

The four highlights are 0.4-era features (keybindings tab, collapsed-section
persistence, error banners, AccessKit toggle). This is the first thing a new
user sees, on every tab.

**Fixed.** Highlights derived from `CHANGELOG.md`'s user-facing entries
between 0.4 and 1.0 rather than invented: the stable release behind three
candidates and an external audit, native Wayland reaching parity with X11,
config durability, and physical-pixel overlay geometry. The nine
non-English strings are machine translations and want a native-speaker
pass before release.

Fixing it surfaced two adjacent defects, R12 and R13 below, and one design
question worth stating: `should_show()` returns true for `last_seen ==
None`, which is exactly a **fresh install** — so a user who had never run
any version was shown a changelog for a release they were never present
for, on top of the onboarding tour they also get. A config created because
no file existed is now stamped with the current anchor, leaving the panel
to upgraders. A config that exists but fails to parse is not a fresh
install and still sees it.

---

## Rendering

### R5 · `→` renders as a missing-glyph box — `FIXED`

"AccessKit can be turned off from Appearance **□** Accessibility" — the arrow
in `whats-new-accessibility-toggle` has no glyph in the proportional font.

The app bundles no fonts of its own; it uses egui's defaults. Note that `↑`
*does* render in the keybindings chords (monospace), so this is narrower than
"arrows are broken" — it is the proportional face that lacks U+2192. Any
string using `→` in body text is affected.

**Fixed.** Reading the bundled cmap tables settles it: Ubuntu-Light,
NotoEmoji-Regular and emoji-icon-font cover *none* of U+2190..U+2193, while
the bundled Hack face covers all four — which is why the same arrows looked
fine in the monospace keybindings chords. The command palette's "↑↓ + Enter
to pick" footer was affected too, though never observed. `icons::install`
now appends the monospace list to the proportional family as a last-resort
fallback: no new asset, and it covers anything else Ubuntu-Light lacks.

### R6 · Sprites bleed through the settings panel — `BY DESIGN`

Entities positioned under the sidebar show through its background and land
behind the panel's text, on every tab.

**Not a defect.** The panel is deliberately frosted — `panels::settings`
builds its frame at alpha 235/255 with the comment *"the desktop reads
through behind the settings instead of a solid slab"*. Measured with the
near-white demo `star` (RGB 253,248,208) behind it:

| behind the panel | panel background | text contrast |
|---|---|---|
| nothing | (30, 34, 43) | 13.5:1 |
| the star | (77, 67, 44) | **8.1:1** |

Body text still clears WCAG AA (4.5:1) and AAA (7:1), so this is visual
noise rather than a legibility failure. Lowering the alpha is a one-line
change in `panels::settings` if the frosted look is ever judged not worth
the distraction — but that is a taste call, not a bug fix.

### R6b · Panel alpha "does not blend uniformly across channels" — `RETRACTED (measurement error)`

**There is no bug here. This entry was wrong, and it is kept only so the
mistake isn't made a second time.**

It originally read: solving the composite per channel gave effective alphas
of 0.789 / 0.846 / 0.994 against a designed 235/255 = 0.922, therefore the
alpha was being applied in the wrong colour space or applied twice — and it
was filed as a Linux-side confirmation of the external audit's M12.

Two independent errors produced that result.

**1. The wrong colour space — in the analysis, not in the app.** The
composite was solved in **sRGB byte space**. But the render target is an
sRGB format and the pipeline blends premultiplied:

- `wgpu_renderer.rs:322` — `caps.formats.iter().find(|f| f.is_srgb())`
- `wgpu_renderer.rs:409` — `blend: BlendState::PREMULTIPLIED_ALPHA_BLENDING`

With an `...Srgb` target the hardware decodes to linear, blends, and
re-encodes. Solving from raw screenshot bytes as though the blend were
linear-in-bytes is simply the wrong equation, and its error grows with the
distance between the two colours — which is exactly why the three channels
disagreed by so much, and why blue (where sprite and panel nearly match)
looked "correct" while red and green looked broken.

**2. The two pixels were never the same pixel.** The original numbers
compared the *brightest* star pixel found in one screenshot against a
*different coordinate* in another. The star is a gradient, so the "source"
colour fed into the equation was never the colour actually behind the
sampled point.

Re-measured properly — same coordinates in both frames, solved in linear
space — the channels with real signal land on the designed value:

```
 pixel        alpha R / G / B        designed 0.9216
 (1350,520)   0.921 / 0.923 / 0.867
 (1365,515)   0.926 / 0.928 / 0.948
 (1380,530)   0.919 / 0.922 / 1.000
```

R and G agree at **0.919–0.928**. Blue stays noisy because the sprite is
yellow: panel blue (43) and sprite blue (47) are nearly equal, so the
solve's denominator approaches zero and quantisation error explodes. That
is instability in the measurement, not spread in the blend.

A fully controlled capture (animation paused) was attempted to remove the
last variable and **failed**: pausing playback stops animation frames but
not behaviour movement, so the sprites still shift between captures. Anyone
retrying this needs a static scene — gravity and behaviours off — not just
paused playback.

The audit's M12 concern about `pick_alpha_mode` accepting `PostMultiplied`
while the pipeline stays premultiplied is **still open on its own terms**
and still scoped to Windows. It gained no Linux evidence here.

### R7 · Keybindings rows wrap mid-token and overlap — `FIXED`

At a normal panel width (≈460 px on a 1600 px output):

- chords wrap across three lines mid-token — `Ctrl+` / `Shift` / `+H`
- the **"+ Add" button breaks its own label** into `+` / `Ad` / `d`
- that button then **overlaps the chord label**, so e.g. the `↑` of
  "Nudge selection up" is partly hidden behind it

**Fixed.** With that little room egui was breaking text *inside* items.
`TextWrapMode::Extend` on the chips, the add button and the
unbound/recording labels stops any single item breaking, so the row wraps
between whole items instead — which is what `horizontal_wrapped` is for.

### R8 · Banner text is clipped by its close button — `FIXED`

"Tip: V toggles visibility, G toggles gravity — no need to open this panel"
runs into the `✕` with no gap and is cut at the panel edge.

**Fixed.** The body label was added before the close button in a horizontal
layout, so a long hint claimed the whole row and the button was drawn over
it. The button now takes the right edge first and the body wraps into the
width actually left, inside a nested left-to-right layout so wrapped lines
stay left-aligned.

---

### R12 · "Settings split across three tabs" — there are five — `FIXED`

`onboarding-tabs` named Inspector, Scene and Appearance. Library and
Keybindings landed later and the hint was never updated, in any locale.

### R13 · Banner text silently decided the settings panel width — `FIXED`

The what's-new highlight labels did not wrap, so they asked for their
natural width and the `SidePanel` grew to satisfy them. Measured on a
1600px output:

| panel contents | width |
|---|---|
| no what's-new panel | **320px** — the configured `default_width` |
| the old 0.4 copy | 424px |
| longer 1.0 copy | 593px |

So the longest translated string in any locale was deciding how much of
the screen the settings panel took. Wrapping the labels returns it to the
configured 320px. Worth remembering when adding copy: `SidePanel` grows to
fit unwrapped content regardless of `default_width`.

## First-run experience

### R9 · Notices crowd out the settings — `FIXED`

The "What's new" panel renders at the top of **every** tab, not once. Together
with the persistent hint banners ("Settings split across three tabs…",
"Themes apply instantly…", "Press Ctrl+Shift+\` …"), roughly half the panel
height on first run is notices rather than settings.

**Fixed**, in two parts. R4's fresh-install change removes the changelog
for first-time users entirely, and the panel now renders only on the
default tab instead of above every tab body — it used to reappear each
time the user switched tab, competing with whatever they had navigated to.

### R10 · X11-only toggle is live in a Wayland session — `FIXED`

The Scene tab offers "Land on windows (X11)" as an active control under
native Wayland, where window-awareness is inert by design (no EWMH
equivalent). It is labelled `(X11)` but is not disabled or explained.

**Fixed.** Disabled on the backends that cannot serve it, with the reason
stated inline rather than only in a tooltip. Note the test is *not* the
display server: a Wayland session running the app through XWayland reads
EWMH fine, which is the common case on GNOME. The caller passes what its
backend actually has — `true` from winit, `false` from `run_native`.

---

## Fixed

### R11 · Soak metrics never recorded on the native Wayland path — `FIXED`

`SoakRecorder` was constructed and sampled only in the winit render loop, so
`ANIMA_SOAK_METRICS` was a silent no-op under `ANIMA_USE_WAYLAND_NATIVE` —
the file stayed empty with no indication the backend was being skipped.
Wired into `run_native`; rows now appear.

---

## Not defects — recorded so they are not re-investigated

- The demo `star` entity sits at `x = 1300`, so it is off-screen on outputs
  narrower than ~1400 px. The shipped default targets 1920×1080.
- The renderer refuses to start when the surface offers no transparent alpha
  mode (bare Xvfb, no compositor). That is deliberate and the error message
  says so — an opaque overlay would cover the desktop in black.

---

### R14 · Every modifier chord was dead on native Wayland — `FIXED`

The command palette had never been opened in a test. `Ctrl+K` did nothing.

`egui_render.rs` built its `RawInput` with a hardcoded
`modifiers: egui::Modifiers::default()`, so `input.modifiers` was
**permanently all-false** on the native Wayland path. Individual
`Event::Key`s carried the correct modifiers, which is why this survived
review — the events look right; the field egui actually answers
`input.modifiers` from is a different one.

Blast radius is wider than the palette. Anything reading `input.modifiers`
was affected, and that includes egui's own `TextEdit` chords, so **every
text field in the app** had no Ctrl+A, Ctrl+C/V/X, Ctrl+Z and no
shift-selection. It also reached the keybindings tab, which reads
`i.modifiers` when capturing a new chord (`keybindings_tab.rs`) — so
recording "Ctrl+Shift+X" on this backend stored a bare "X". Confirmed both ways: typing `dark`, pressing Ctrl+A, then
typing `zz` left `darkzz` before the fix and `zz` after.

**Fixed** by plumbing the live seat state through (`LayerWindow::modifiers`)
and, on top of that, preferring the modifier snapshot carried by the newest
key event in the frame (`effective_modifiers`). The second half matters on
its own: sampling only live state loses a chord whose press *and* release
both land inside one frame, which any long frame — a video or GIF decode
stall — makes possible. Unit tests cover both directions.

Two things this cost, worth writing down:

- **`wtype` is the keyboard's version of the `wlrctl` trap.** It creates a
  virtual keyboard, sends its keys and exits, so the seat's keyboard
  capability blinks in and out. The app binds `wl_keyboard` from
  `SeatHandler::new_capability`, which is asynchronous and never completes
  in time, so **not one key is delivered** and nothing is logged. The rig
  now holds a `wtype -s 3600000 -k Shift_L -k Shift_L` open for the whole
  run purely to keep the capability up.
- **`wtype` releases modifiers immediately**, faster than a frame, so
  `-M ctrl -k k -m ctrl` reproduces the stall case rather than normal
  typing. Use `-M ctrl -P k -s 800 -p k -m ctrl` to emulate a human hold.

### R15 · Inactive tab icons fail contrast in the Light theme — `FIXED`

Found on the first look at a non-dark theme, reached through the command
palette that R14 had just made usable.

The settings tab bar is **icon-only**, so each glyph is the whole label of
its control and owes WCAG 1.4.11's 3:1 for non-text UI components. Inactive
icons paint in `palette.fg_muted`:

```
                design    as rendered
 dark   #6B7280  3.29:1      3.17-3.37:1   pass
 light  #9CA3AF  2.29:1      2.11:1        fail
```

The rendered figures sit slightly below the design ones because the frosted
panel blends toward the desktop behind it.

There were already contrast assertions in `theme.rs`, but only for the two
*high-contrast* palettes — the ordinary Light and Dark themes had none for
`fg_muted`, so nothing caught this.

**Fixed.** Light `fg_muted` is now `#7C838F` (3.44:1 design, 3.04–3.17:1
rendered — parity with dark), and a test asserts ≥3:1 for both ordinary
palettes. Verified the test fails on the old value and passes on the new.

`fg_muted` has exactly one use outside `theme.rs`, this tab bar, so the
change is contained.

### R16 · Keybinding rows paint over each other — `FIXED`

Found while checking German for layout damage; it turned out not to be a
locale problem at all — English had it just as badly.

The tab was an `egui::Grid` whose middle column is a `horizontal_wrapped`
run of chord chips plus the "+ Add" button. The cell reserved height for
fewer lines than it went on to paint, so the *next* row's stripe was drawn
straight over the tail of the previous one.

Not cosmetic: for every action with two chords the "+ Add" button was
entirely covered — invisible and unclickable — and a chord chip was sliced
in half.

**Fixed** by dropping the grid for one `Frame` per action. A frame reserves
its background shape and fills it after laying out its contents, so the
stripe can never be shorter than what it sits behind. The label column now
wraps inside a fixed width, which also stops German widening the whole side
panel past its English width.

### R17 · Ten complete locales, English on screen — `FIXED`

The behaviour picker — Idle / Walk around / Follow cursor / Bounded wander
/ Bounce — drew hardcoded English in **every** locale, while
`behavior-idle`, `-walk`, `-follow`, `-wander` and `-bounce` sat translated
in all ten `.ftl` files, unused. Same for the Appearance theme row, the
inspector's X / Y labels, and two Dismiss tooltips.

`every_locale_covers_every_en_key` proved the translations existed. Nothing
proved the app ever asked for them, so this was invisible to the suite.

**Fixed**, and the gap closed with `every_en_key_is_referenced_in_the_source`
— it scans the crate for each English key as a string literal. Sound here
because keys are only ever named by literal; `Action::i18n_key` and friends
return `&'static str` out of a match.

That test then found four more keys, all dead rather than hardcoded:
`palette-close-hint` and `palette-apply-preset` were superseded by
`palette-footer-hint` and the Replace/Append rows, and
`library-sort-name` / `library-sort-recent` describe a sort control that
does not exist (`library.rs` contains no "sort"). Removed from every locale
rather than allowlisted, so the test stays strict.

### R18 · Clicking the settings panel throws away the selection — `FIXED`

Select a character, click any blank part of the settings panel, and the
Inspector drops back to "Nothing selected" — the click fell through to the
scene's hit test, found nothing, and deselected. The panel you are reading
is the thing that clears itself.

**This one was mine**, introduced with the R2/R3 fix that gave this backend
left-click selection and drag. The winit path never had the bug because
`App::window_event` hands every event to egui first and returns early when
it is consumed; the native Wayland loop reads the raw event list and had no
equivalent.

**Fixed** with `WaylandEguiRenderer::owns_pointer()`
(`Context::is_pointer_over_area()`) gating the two press arms. Only presses
are gated — motion and release stay live so a drag begun on a sprite still
tracks and still finishes if the pointer crosses the panel. Verified all
three ways: selection survives a panel click, sprites still select, and a
dragged character lands on the exact target.

### R19 · 27 of 28 keybindings do nothing on native Wayland — `FIXED`

Select a character, press the arrow keys bound to "Nudge selection". It
does not move. Verified with the character confirmed selected — the
Inspector showed "Cat Demo" throughout — so this is not a selection
problem.

The cause is a parity gap, not a bug in the key path. Keys arrive fine
(R14 fixed that). But `src/wayland/run.rs` consults the keybinding table
in exactly one place and matches exactly one action:

```rust
if let Some(Action::ToggleEditMode) = config.keybindings.lookup(chord) {
```

The winit/X11 path routes through `App::dispatch_action`, which handles
**27** actions. Native Wayland handles **one**.

So the Keybindings tab lists ~28 rebindable actions, lets the user rebind
them, and persists the result to `config.toml` — and on this backend all
but one are decorative. Nudge, delete, cycle, centre, the visible/gravity/
playback toggles, opacity, FPS, duplicate, z-order, save, quit: none of
them fire.

Ctrl+K is the exception that hides it, and only by accident — the command
palette reads `ctx.input()` inside egui rather than going through the
table at all.

**Fixed** by the split this entry predicted, measured rather than guessed:
of the 26 arms, **18** touch only the scene, the selection and the dirty
flag, and **8** genuinely need a window, a renderer or the event loop.

The 18 moved to `keybindings::shared::dispatch_shared`, which both backends
now call; it returns `false` for anything it does not own so each caller
still runs its own arms. Native Wayland went from 1 handled action to 19.

Verified on the rig rather than by reading: five presses of Right moved a
character exactly 50 px, `V` took it from 1199 visible pixels to 0 and back
to 1168, and `Tab` moved the selection from Cat Demo to Ghost Demo.

Still per-backend, because they differ for real: quit-with-save,
save-now, edit-mode toggle, delete, centre-on-screen, duplicate (it goes
through the renderer's texture cache), cycle-monitor and the perf overlay.
Those remain unavailable on native Wayland and are worth a follow-up.

This sat inside the project's standing X11/Wayland parity exception
(`CONTRIBUTING.md`, granted 2026-06-21).

### R20 · `Single` monitor mode ignores the chosen monitor on Wayland — `FIXED`

Set `monitor_mode = single` with `name = "HEADLESS-1"` (the right-hand
output of two) and the overlay renders entirely on the *left* one. Counted
directly: 5704 non-black pixels on the left, **zero** on the right.

The primary layer surface is created with no output:

```rust
let layer = layer_shell.create_layer_surface(
    &qh, wl_surface, Layer::Overlay, Some("anima_engine"),
    // Any output — the compositor picks, same as the X11 path
    // never explicitly positions its primary window either.
    None,
);
```

That justification is **stale**. The X11 path *does* position its primary
window now — `app/windows.rs` says so explicitly, and records that not
doing it was the bug which made "selecting a non-primary monitor in
`Single` mode silently do nothing". The same bug is still here, kept alive
by a comment pointing at behaviour that has since changed. Same shape as
the `_arc_used` placeholder whose stated reason had also stopped being
true.

**Fixed.** A layer surface's output is fixed at creation and the output
list has not arrived by then, so `try_create` now takes the monitor name
`Single` asks for, does one extra round-trip to learn the outputs, and
replaces the surface with one bound to the requested output. The
replacement is built only when a specific monitor was asked for and the
first surface is not on it — every other mode keeps the compositor's
choice and costs nothing.

The swap happens *before* the wgpu surface is built, so the careful
drop-order invariant documented around `build_wgpu_surface` is untouched:
nothing references the discarded surface.

Verified on the rig, exactly inverting the original measurement — with
`Single`/`HEADLESS-1` on two outputs there are now **0** non-black pixels
on the left and 6222 on the right. A stale monitor name warns and falls
back to the compositor's choice rather than failing to start, and a normal
single-output run is unchanged.

### R21 · `Span` covers one monitor on Wayland, but says "all" — `FIXED`

With two outputs and `monitor_mode = span`, the renderer initialises at
1600×1000 — one output — and a character placed at x = 2100 never appears.

Unlike R20 this is arguably inherent: a `wlr-layer-shell` surface belongs
to an output, so there is no single whole-desktop surface to create. The
code knows: `monitor.rs:313` says "`Span` draws **one** window sized to a
single monitor".

The defect is that nothing tells the user. The picker offers **"Span all
monitors"** and `docs/engine-features.md` promises "one overlay spanning
all monitors" — on this backend it silently means "one monitor, and your
characters on the others vanish".

**Fixed the honest way**, as R10 did for window-awareness: the mode is
offered disabled on backends that cannot span, with a tooltip pointing at
per-monitor instead. `span_supported` is plumbed separately from
`window_awareness_supported` even though both are false on the same
backend today — they are different capabilities, and conflating them would
mislead whoever adds the next backend.

Deliberately still available with a **single** monitor, where "span all"
and "cover this one" are the same thing and the label is not a lie. It only
misleads once a second monitor exists. `engine-features.md` now says X11
only rather than promising it flatly.

### R22 · A recovered startup condition is logged at ERROR — `OPEN`

`get_physical_device_surface_capabilities: ERROR_SURFACE_LOST_KHR` is
logged at `ERROR` on every launch, single- and multi-output alike, and the
renderer initialises fine on the very next line.

**Correction on first writing this up:** the message is not ours. It does
not appear anywhere in `src/`, so it is wgpu's own log reaching the
terminal through our `tracing_subscriber`. That changes the fix from "move
one log line" to a judgement call:

- narrowing the default `EnvFilter` to quieten that wgpu target would stop
  the noise, but suppressing a graphics library's `ERROR` wholesale is a
  good way to miss a real device failure later;
- leaving it means every launch prints an `ERROR` the code handled, which
  is exactly how people learn to skim past the level that matters.

**Investigated, not resolved.** The hypothesis was multi-GPU adapter
probing: this machine exposes three Vulkan devices (RADV, NVIDIA,
llvmpipe), and wgpu asks each whether it supports the surface, so one
incompatible adapter answering `SURFACE_LOST` while another succeeds would
explain it entirely — and make it upstream noise rather than ours.

It could not be confirmed. Restricting `VK_DRIVER_FILES` to a single ICD
does make the message disappear, but every single-ICD configuration then
fails for an unrelated reason on this rig — llvmpipe and RADV both report
only `Opaque` composite alpha under headless sway, so the renderer refuses
before it would have logged anything. The two observations are therefore
not comparable, and "it goes away with one driver" proves nothing.

So it stays open, and the honest position is that we do not yet know
whether this is benign. It should not be filtered away on a guess: a
graphics library's `ERROR` is exactly what you want to still be reading
the day a device really is lost. Confirming it needs either a
single-GPU machine where the app actually runs, or wgpu-side logging of
which adapter produced it.

### R23 · Japanese renders as boxes, end to end — `FIXED`

Start the app with `LANG=ja_JP.UTF-8` and **every Japanese character in
the UI is a missing-glyph box**. Not one label is readable. Latin text in
the same panels — `PNG`, `Ctrl+Shift+A`, `portal (GlobalShortcuts)`,
`config.toml` — renders fine, so this is purely a script-coverage gap.

Polish was checked alongside it and is perfect, diacritics included (ó, ą,
ę, ś, ż, ł), so Latin-extended coverage is not the issue. Japanese is the
only CJK locale shipped, so it is the only one affected — and it is
affected completely.

This is the same family as R5, where `→` had no glyph in the proportional
font, but the consequence is a different order of magnitude: R5 cost one
arrow, this costs an entire advertised language. The README promises "ten
UI locales"; nine of them work.

The bundled stack is egui's Ubuntu-Light → NotoEmoji → emoji-icon-font,
plus the Hack monospace face appended by the R5 fix. None carries CJK, and
no amount of re-ordering fixes that — the glyphs are simply not there.

Options, none of them free:

- **Bundle a CJK face.** Correct everywhere, offline, deterministic. Noto
  Sans JP is several MB even subset, against a 24 MB binary, and it only
  solves Japanese — Chinese or Korean later would want more.
- **Load a system font at runtime.** The test machine has 31 CJK faces
  installed, so on a desktop where someone actually reads Japanese one is
  almost certainly present. Needs a font-discovery dependency (none in the
  tree) and degrades to today's behaviour when nothing is found — which
  argues for pairing it with the next option.
- **Say so.** Whatever else is done, the language picker should not offer a
  language that cannot be drawn. Disabling it where no CJK face is
  available is the same honesty R10 applied to window-awareness.

**Fixed** with the second and third together. A short list of well-known
CJK font paths is probed and the first hit loaded — no font-discovery
dependency, which would have been a large tree for one lookup per session.
Loaded *only* when the active locale needs it, since the face is ~19 MB and
holding that for someone reading English buys nothing, and re-installed
when the language changes so switching at runtime works rather than
silently keeping the Latin-only stack.

When no CJK face exists, the picker offers those languages disabled with a
tooltip naming the missing package, instead of letting someone select a UI
they can no longer read — including the picker itself.

Verified on the rig: `LANG=ja_JP.UTF-8` now renders "インスペクター",
"何も選択されていません" and the full hint text.

### R24 · Romanian uses three words for the same thing — `FIXED`

The Keybindings panel titles itself **"Comenzi taste"**, the banner under
it says **"Scurtături"**, and the body text mixes both with
**"combinație"**. Counted across `ro.ftl`: "scurtături" 8×, "comenzi" 6×,
"combinație" 4× — for one concept, on one screen. **"chord" is left in
English** twice, which is jargon even in English.

Nothing renders wrong; the diacritics are all correct. This is the
machine-translation debt catching up, and it is worth a finding rather
than another generic "the nine locales want a native pass" line, because
it is now demonstrable rather than assumed.

**Correction to the count above.** Six of those "comenzi" are *correct*:
"paleta de comenzi" is the command palette, which really is a list of
commands. The inconsistency is narrower than first written — "comenzi"
used for *keybindings* while "scurtături" is used for the same thing
elsewhere.

**Fixed** on that narrower reading. "Scurtături" is the term for the
feature (matching the Windows and GNOME Romanian conventions),
"combinație" for the key sequence itself — a distinction worth keeping,
not collapsing — and "chord" is gone. The command palette strings are
untouched.

The other eight non-English locales have had no such check and very likely
carry the same kind of drift. Terminology is the maintainer's call; this
is one line per string to revise if a different word is preferred.

## Still unexamined

Verified since, on the headless rig:

- **Scripted behaviors run live.** A character with `type = "script"` moved
  453 px in two seconds against 440 expected at `speed = 220`. The asset
  library is also created on first run now, which it was not.
- **Multi-monitor (PerMonitor).** With two outputs the app spawns an extra
  layer surface on the second and a character placed at x = 2100 renders
  there, not on the primary.
- **Both high-contrast themes.** Tab icons measure 14.7–15.9:1 on dark HC
  and 8.6–11.7:1 on light HC — comfortably past AAA, unlike the ordinary
  light theme, which needed R15.
- **Keyboard shortcuts end to end** — which is how R19 was found.

Also verified: **preset Append and Replace** both behave — Append took the
scene from 5 entities to 6, Replace took it to 1, and the footer even
pluralises "1 entity" correctly.

Also swept: **all ten locales**. Every Latin-script one renders correctly,
diacritics included; Japanese was R23. Romanian turned up a translation
consistency problem rather than a rendering one — R24.

**Shimeji pack import** was exercised for the first time, including its
hardening. A legitimate pack imports (actions.xml parsed, `Stand` → idle,
`Walk` → walk, frames copied, missing states reported with reasons), and
three hostile packs are all refused with nothing from outside the pack
reaching disk: an `actions.xml` that is a symlink to `/etc/passwd`, a
sprite path of `../../../../etc/hostname`, and a FIFO where a sprite
should be. The threat-model claims there hold up.

Still unexamined: drag-and-drop of files onto the overlay, and the whole
winit/X11 path interactively — Vulkan reports only `Opaque` composite alpha under
Xvfb+picom with both the NVIDIA and software drivers, so the renderer
refuses by design and the path cannot be driven here.

The rig now drives the keyboard as well as the pointer, which is what made
R14 findable. Two traps in doing so are written up under R14; the short
version is that both `wlrctl` and `wtype` create their virtual device, use
it and exit, and the app can never bind a device that transient.
