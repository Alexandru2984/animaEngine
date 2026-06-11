# Road to 1.0 — engineering plan

Detailed companion to [ROADMAP.md](../ROADMAP.md). Four releases
separate 0.5.5 from 1.0; each has a single theme, a fixed sub-phase
list, and a definition of done. Sub-phases are sized S/M/L (hours /
day / multi-day), land as one commit each, and every one passes the
standard gate before merging:

```
cargo fmt --all
cargo clippy --lib --bin anima_engine --tests -- -D warnings
cargo test --lib
```

Release ceremony (unchanged from 0.5.x): security audit against the
unreleased tree → CHANGELOG entry → `docs/release-notes/vX.Y.Z.md` →
metainfo `<release>` entry → version bump → `.deb` + AppImage +
SHA256SUMS → tag → GitHub release → Flathub manifest bump.

## What 1.0 promises

1.0 is a contract, not a number:

- **Config stability** — a `config.toml` written by 1.0 loads in every
  1.x; migrations are automatic and tested.
- **No half-built surfaces** — nothing visible in the UI is stubbed,
  and no shipped code path carries a "lands later" comment.
- **First-class on the big three** — Ubuntu GNOME (X11 + Wayland),
  KDE Plasma, wlroots compositors; global shortcuts work natively on
  all of them.
- **Endurance** — a documented multi-day soak with flat memory.
- **Reachable** — installable from GitHub Releases, Flathub and AUR.

Explicitly *not* in 1.0: macOS / FreeBSD ports, plugin scripting
(WASM/Lua), instanced rendering / texture atlas, any network feature.

---

## 0.6 — Platform completeness (Faza T)

Theme: the engine works the same everywhere it claims to run.
Closes the two oldest platform gaps: global shortcuts on GNOME/KDE
Wayland sessions, and the multi-monitor data model that has shipped
since 0.3 without a render-side implementation.

| # | Sub-phase | Size | Detail |
|---|-----------|------|--------|
| T.0 | Portal probe + decision log | S | Detect `org.freedesktop.portal.GlobalShortcuts` on the session bus (version, backend). Log the chosen hotkey strategy at startup: portal / XGrabKey / D-Bus-only. No behavior change. |
| T.1 | Portal client module | M | `src/hotkeys/portal.rs`: zbus proxy for CreateSession → BindShortcuts → Activated/Deactivated signals. Session restore token persisted in config so rebinding survives restarts. Bounded signal→event channel (same pattern as the D-Bus service, `sync_channel(64)`). |
| T.2 | Event wiring + fallback chain | M | Portal `Activated` → `AnimaEvent` (existing variants). Strategy selection: portal when available → XGrabKey on X11 → tray/D-Bus only, with the warning banner downgraded accordingly. Config knob `global.hotkey_backend = "auto" \| "portal" \| "x11" \| "none"`. |
| T.3 | First-run permission UX | S | The portal pops a system permission dialog on first bind. Toast before, explanatory hint in Keybindings tab, graceful handling of denial (banner + tray fallback). |
| T.4 | Keybindings tab portal state | S | Show which backend is live; portal rebinds call `BindShortcuts` again; X11-only chords greyed out with tooltip on portal sessions. |
| T.5 | Multi-window architecture decision record | S | Doc + types: one shared `wgpu::Instance` + `Device`/`Queue`, one `Surface` + swapchain config per window. `WindowId → MonitorBinding` map. Records why (texture cache shared across monitors; one device loss domain). |
| T.6 | Window-per-monitor spawn (X11 path) | L | `MonitorMode::PerMonitor` creates one overlay window per detected monitor; `Span` keeps today's single window; `Single{name}` targets one. Per-window XShape input regions. |
| T.7 | Renderer multi-surface support | L | `WgpuRenderer` splits device-shared state from per-surface state. Draw list partitioned by entity→monitor resolution (the 0.3 data layer finally consumed). egui stays on the primary window only. |
| T.8 | Input routing + edit-mode scoping | M | Edit mode is global; events route per window; selection hit-tests in window-local coordinates; toggle button on every window, panel on primary. |
| T.9 | Monitor hotplug | M | winit monitor-change events → diff against window map → spawn/despawn windows, re-resolve entity pins, toast summary. Wayland native: `wl_output` add/remove already tracked — same diff applied to layer surfaces. |
| T.10 | Suspend/resume + pacing resync | S | After resume, `Instant` jumps: animations resync via the existing two-loop cap; verify `WaitUntil` deadlines in the past fire immediately; surface `Lost`/`Outdated` recovery exercised. Manual test protocol documented in `docs/soak-testing.md` (created here, extended in 0.9). |
| T.11 | Release 0.6.0 | S | Ceremony. Audit focus: portal session handling (token storage), multi-window input-region regressions. |

**Definition of done:** global shortcuts work on stock Ubuntu GNOME
Wayland without XWayland tricks; three monitors show three overlays
with entities pinned correctly; unplugging one mid-session relocates
its entities with a toast; suspend/resume needs no restart.

**Risks:** portal backends differ (GNOME asks once, KDE shows an
editor); multi-window touches the renderer's core — T.7 lands behind
the existing `MonitorMode` so `Span` (default) stays on today's path
until proven.

---

## 0.7 — Content & feature completion (Faza U)

Theme: things users *see*. The Shimeji ecosystem is the user magnet —
thousands of community packs with no maintained Linux engine — and it
forces the one real engine gap: multi-state animations.

| # | Sub-phase | Size | Detail |
|---|-----------|------|--------|
| U.0 | Shimeji format research doc | S | `docs/shimeji-import.md`: pack layout (`img/shime*.png`, `conf/actions.xml`, `conf/behaviors.xml`), which subset maps cleanly, what gets dropped (window-climbing, multi-mascot interactions). Caps: max pack size, max images, XML entity-expansion guards. |
| U.1 | Multi-state animation model | L | `AnimationSet`: named states (`idle`, `walk`, `fall`, `drag`) each with its own frame sequence + fps; `Behavior` drives state selection; config schema additive (`[characters.animations.idle]`…). Single-sequence configs keep loading unchanged — the old shape maps to a one-state set. |
| U.2 | Behavior ↔ state wiring | M | Walk plays `walk` facing movement direction (horizontal flip), gravity fall plays `fall`, drag plays `drag`, default `idle`. Texture upload path handles per-state dimension changes. |
| U.3 | Shimeji pack importer (core) | L | Parse pack → N `CharacterConfig`s with animation sets. Hardened XML parsing (`quick-xml`, no DTD, depth/size caps), image caps reuse the asset pipeline, paths canonicalised under the import root. Pure function + golden-file tests against 2–3 reference packs. |
| U.4 | Import UX | M | Library tab "Import Shimeji pack" — drag a pack folder onto the overlay or paste a path (no new file-dialog dependency). Progress toast, per-pack error report, imported packs land in the asset library as first-class entries. |
| U.5 | Library thumbnails (closes C.5) | M | First decoded frame → 64 px PNG in the existing `thumbs/` cache dir (paths shipped in 0.3, never populated). Decode off the UI thread through the drop-worker pattern; egui texture cache with eviction; cache key = the 0.5.5 fingerprint. |
| U.6 | Library grid v2 | S | Thumbnail grid replaces text rows; fallback glyph for un-decodable entries; keyboard navigation preserved (a11y). |
| U.7 | Group composition in renderer (closes C.9) | M | `offset_x/y` + `scale` from `GroupConfig` applied at draw-list build; inspector shows effective (composed) transform; hit-testing uses composed bounds. |
| U.8 | Release 0.7.0 | S | Ceremony. Audit focus: XML parser surface, importer path handling. Demo pack (self-made, licensed) added to the README walkthrough. |

**Definition of done:** a stock Shimeji pack imports in under 10 s,
walks with the correct state animations, and shows thumbnails in the
library; groups visually compose; zero "lands in C.x" comments left
in the tree.

**Risks:** multi-state animation touches `Animation`, the cache
format (per-state sequences) and the config schema — U.1 is the
release's load-bearing wall and lands first with exhaustive tests.
Cache format bump = one-time rebuild, same as 0.5.5.

---

## 0.8 — UI/UX polish (Faza V)

Theme: the long-promised dedicated polish phase. No new capabilities;
every screen gets deliberate attention. Each sub-phase starts from a
written audit, not vibes.

| # | Sub-phase | Size | Detail |
|---|-----------|------|--------|
| V.0 | Heuristic audit | M | Walk every surface (both themes × HC variants, two locales, keyboard-only). Output: `docs/ux-audit-0.8.md` with numbered paper cuts, each tagged fix/wontfix. The release scope *is* this list. |
| V.1 | Motion & transitions | M | Panel slide/fade, tab cross-fade, toast spring — all through `crate::anim` curves; honors a reduced-motion setting (new a11y toggle, also respected by entity idle bobbing). |
| V.2 | First-run experience | M | Empty desktop on first launch: offer the demo scene + a 3-step coach-mark tour (replaces the static onboarding hints). Dismissible forever, re-armable from Appearance. |
| V.3 | Inspector ergonomics | M | Logical control grouping, units on sliders, double-click-to-reset, drag-value acceleration, per-section reset. |
| V.4 | Keyboard & a11y re-audit | M | Focus order on every panel, visible focus on custom widgets, AccessKit tree re-validated with Orca, contrast re-check after V.1 visual changes. |
| V.5 | Visual consistency pass | S | Spacing scale violations from V.0, icon sizing, design-system.md updated to match reality. |
| V.6 | i18n for new strings | S | Every string added in 0.6–0.8 through the native-review pipeline; locale audit doc refreshed. |
| V.7 | Release 0.8.0 | S | Ceremony. Audit focus: none new (no surface change) — regression audit only. |

**Definition of done:** the V.0 list is empty or explicitly wontfixed;
keyboard-only and screen-reader walkthroughs complete without dead
ends; reduced-motion honored everywhere.

---

## 0.9 — Stability freeze (Faza W)

Theme: prove the contract before promising it. Feature work stops at
W.0; everything after is measurement, hardening and paperwork.

| # | Sub-phase | Size | Detail |
|---|-----------|------|--------|
| W.0 | Config schema versioning | M | `version = 2` field (v1 = implicit pre-0.9), migration registry (`v1→v2`, applied in order, `config.toml.bak` before), round-trip tests per migration, forward-compat policy: unknown keys preserved on save. |
| W.1 | Soak harness | M | `scripts/soak.sh`: Xvfb + synthetic 16-entity scene, samples RSS + `total_decoded_bytes` + texture count via perf export every minute for N hours, emits a markdown report. CI job runs the 30-minute variant nightly. |
| W.2 | Leak hunt | M | Fix whatever W.1 finds (suspects: egui texture deltas, toast queue, monitor-hotplug window maps). Soak re-run proves flat. |
| W.3 | Perf HUD completion | M | VRAM estimate (decoded bytes + texture bytes), texture uploads/frame, draw-call count, GPU pass time via `TIMESTAMP_QUERY` where the adapter supports it. The overlay becomes the soak's visual twin. |
| W.4 | Fuzz expansion | M | New targets: `cache::deserialize_frames`, `avcc_to_annex_b`, Shimeji XML importer. Seed corpora from real assets; nightly short-run CI job; `docs/fuzzing.md` updated. Closes the threat-model TODO. |
| W.5 | Docs reality audit | S | Every file in `docs/` diffed against the code it describes; architecture.md regenerated diagrams; config.md covers every field including animation sets. |
| W.6 | Full locale audit | M | The 0.4 cross-locale AI+native pipeline re-run over the complete string set; RTL smoke check. |
| W.7 | Freeze declaration | S | `CONTRIBUTING.md` gains the freeze rules: bugfix-only to 1.0, what qualifies, how exceptions get decided. |
| W.8 | Release 0.9.0 | S | Ceremony. Audit: full-tree re-audit (the 1.0-blocking one). |

**Definition of done:** 7-day desktop soak documented with flat
memory; migrations tested both ways; fuzz targets running nightly
with zero findings outstanding; docs match code.

---

## 1.0 (Faza X)

| # | Sub-phase | Size | Detail |
|---|-----------|------|--------|
| X.0 | RC1 | S | Tag `v1.0.0-rc1`, build artifacts, two-week bake: RC on the daily-driver desktop + soak rig. Only RC-blocker fixes land. |
| X.1 | The contract document | S | `docs/stability-policy.md`: what 1.x guarantees (config compat, D-Bus interface stability, CLI flags), what it doesn't, deprecation process. Linked from README. |
| X.2 | Distribution verification | M | Flathub listing live and installing; AUR package voted/building; `.deb`/AppImage on the release. Install-from-scratch walkthrough tested on a clean VM for each channel. |
| X.3 | README final | S | Demo GIF top-fold, badges (CI, release, Flathub), screenshots, 1.0 positioning rewrite. |
| X.4 | v1.0.0 | S | Ceremony + "what 1.0 means" release notes + announcement post (blog #2: the road from toy to 1.0). |

**Definition of done:** a stranger with a clean Ubuntu install is
running animaEngine with working global shortcuts within five
minutes, from any of three install channels, and their config will
survive every 1.x upgrade.

---

## Cross-cutting rules

- **One sub-phase = one commit**, conventional message, no phase
  numbers in the message, full gate before each commit.
- **Audit before every tag** — the 0.5.x ceremony is permanent.
- **Anything moved out of a release gets written down** in ROADMAP.md
  non-goals or the next release's list — scope is explicit, always.
- **User-blocking items stay parallel-tracked**: demo GIF, Flathub
  submission, AUR publish, blog posts. None of them gate code work;
  all of them gate 1.0 (X.2/X.3).
