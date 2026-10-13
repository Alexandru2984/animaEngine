# Soak & resilience testing

Started in 0.6 (T.10) with the suspend/resume + stall protocol; the
long-run soak harness shipped in 0.9 (W.1) and extends this file.

## Time bases — read this first

On Linux, `std::time::Instant` is `CLOCK_MONOTONIC`, which **does not
advance during suspend**. Consequences for animaEngine:

- **Suspend/resume creates no animation backlog.** Frame clocks and
  `ControlFlow::WaitUntil` deadlines freeze with the system and
  continue seamlessly — there is nothing to "catch up".
- The case that *does* stress the pacing/backlog machinery is a stall
  where monotonic keeps running: `SIGSTOP`/`SIGCONT`, a debugger
  pause, or an overloaded machine. That's what the two-loop resync
  cap in `Animation::tick` absorbs (frames walk at most two loops,
  then the clock resyncs).

So the suspend protocol below mostly exercises the *display stack*
(surface loss, compositor restarts, portal session survival), while
the stall script exercises the *timing* machinery.

## Stall test (scripted, runs anywhere)

```bash
cargo build
RUST_LOG=anima_engine=debug timeout 25s ./target/debug/anima_engine &
APP=$!
sleep 4
kill -STOP $APP; sleep 10; kill -CONT $APP
sleep 5; kill $APP 2>/dev/null
```

Pass criteria:

- process alive after `CONT`, exits cleanly on TERM;
- no `panicked` in the log;
- animations resume at the correct frame (visually: no fast-forward
  burst — the resync cap converts the backlog into one clean step).

Run on every release that touches `animation/`, the render loop or
pacing. Last run: 0.6 development, pass.

## Suspend/resume protocol (manual, once per release)

1. Start the overlay with an animated scene (≥2 entities, one GIF
   with per-frame delays) and the perf overlay open.
2. Suspend (`systemctl suspend`), wait ≥ 5 minutes, resume.
3. Check, in order:
   - [ ] overlay still composited (sprites visible, no black window);
     a `SurfaceError::Lost/Outdated` recovery in the log is fine, a
     panic is not;
   - [ ] animations advance normally (no burst, no freeze);
   - [ ] pacing intact: with the scene static, CPU returns to ~0%
     between heartbeats (watch `top` for 10 s);
   - [ ] edit mode toggles; XShape click-through still correct
     (Mutter occasionally clips shapes across suspend — the
     `Focused(true)`/`Occluded(false)` re-apply hooks cover it);
   - [ ] global hotkeys still fire (XGrabKey survives; on portal
     sessions check the portal session survived — `ShortcutsChanged`
     / re-bind on demand is the recovery path);
   - [ ] hot-reload still applies (touch config.toml, watch for the
     reload toast within ~2 s).
4. With PerMonitor + a second monitor: repeat step 3's first two
   checks per window, then unplug/replug the secondary while
   suspended — on resume the hotplug diff must rebuild windows and
   toast the pin changes.

## DPMS / screen-blank

`xset dpms force off`, wait 30 s, wake. Expect `Occluded(true)` →
pacing keeps ticking (scene time advances), `Occluded(false)` →
shape re-applied, one redraw requested. No special handling needed;
listed because compositors differ in what they deliver.

## Compositor restart (X11)

`killall -HUP picom` (or restart Mutter with Alt+F2 `r` on Xorg
GNOME): expect transient surface loss, automatic reconfigure, no
crash. On the alpha-mode fail-fast path: if the compositor comes
back *without* compositing, the renderer init would refuse — that
only matters at startup; a running instance keeps its surface.

## Memory soak harness (W.1, automated)

`scripts/soak.sh` runs a 16-entity synthetic scene (mixed demo assets,
behaviours on so the render loop never idles) under Xvfb and regresses
resident-set size against time to catch slow leaks the unit tests
can't.

```bash
scripts/soak.sh [DURATION_SECS] [INTERVAL_SECS] [RSS_SLOPE_KIB_PER_MIN]
# defaults: 1800 (30 min), 60, 512 KiB/min
```

How it works:

- The running app emits a CSV row per interval when
  `ANIMA_SOAK_METRICS=<path>` is set (the env-gated `soak` module).
  Columns: `elapsed_secs, rss_kib, decoded_bytes, texture_count,
  frame_p95_us`. Off and zero-cost in normal runs.
- The script least-squares-fits RSS over elapsed time; a slope below
  the threshold is **FLAT**, above is **DRIFT**. It writes
  `build/soak-report-<date>.md` and exits non-zero on DRIFT (or on
  fewer than three samples), so CI can gate.

CI runs a 10-minute variant nightly (`soak` job, schedule-only) and
uploads the report. The threshold there is 1024 KiB/min over ~30
samples on llvmpipe.

### Unattended runs (the 7-day homelab soak)

The script is safe to leave running for a week on a machine you are not
watching:

- **Bounded output.** The CSV is one row per interval (about 10,000 rows,
  under 1 MB, for a week at 60 s). The app's log goes to
  `build/soak-<date>.log` and is rotated at `SOAK_LOG_MAX_MB` (default
  20) to a single `.old`, so it stays under twice that at any log level.
  The app logs at `warn` unless `SOAK_RUST_LOG` says otherwise; the
  numbers come from the CSV, not the log.
- **Isolated.** It starts its own Xvfb when `DISPLAY` is unset, and its
  own D-Bus session bus. On a shared bus an overlay already running
  would win the single-instance handshake and the soak would measure
  nothing.

```bash
sudo apt install xvfb dbus mesa-vulkan-drivers   # software GPU, as in CI
env -u DISPLAY -u WAYLAND_DISPLAY \
    LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe \
    scripts/soak.sh 604800 60 512      # 7 days, one sample a minute
```

Run it in `tmux` or `screen` so it survives the SSH session. The report
and the CSV are what to hand over for the graphs.

### W.2 leak audit (boundedness ledger)

The short soak ran flat, so W.2 was a proactive code audit of the
pre-registered suspects plus the other long-session growth candidates.
Findings — each is bounded with the cited mechanism:

| Suspect | Bound / mechanism | Evidence |
|---|---|---|
| egui texture deltas | every `textures_delta.free` id is released | `ui/egui_renderer.rs` free loop, unconditional |
| Toast queue | hard cap `MAX_TOASTS = 8`; oldest evicted on push, expired pruned per frame | `ui/toasts.rs`; tests `cap_drops_oldest`, `prune_removes_expired` |
| Hotplug window registry | `extra_windows` cleared before each rebuild (no stale slots); entity textures pruned every frame | `app/windows.rs::rebuild_extra_windows`, `renderer::prune_stale_textures` |
| Portal session | one `CreateSession` at startup, no re-bind loop (T.4 deferred) | `hotkeys/portal.rs::spawn_bg` (single call) |
| Hot-reload threads | one worker at a time (`hot_reload_rx.is_some()` gate); self-terminating after send | `app/hot_reload.rs::check_hot_reload` |
| Perf ring buffer | fixed `capacity` (1024), evicts oldest | `perf.rs::end_frame` |
| Library thumbnail textures | egui memory cache capped at `THUMB_TEXTURE_CAP = 256` | `ui/panels/library.rs` |
| **On-disk decode cache** | **was unbounded** — fixed in W.2 | `animation/cache.rs::sweep`, 1 GiB cap, oldest evicted at startup |

The only real growth found was the **on-disk** decoded-frame cache:
the content-addressed keying orphans a file whenever an asset's
mtime/size changes, and nothing ever reclaimed them. `cache::sweep`
(run once at startup, off-thread) now evicts oldest-first past a 1 GiB
cap. No unbounded *memory* growth was found.

When the harness flags DRIFT despite this, the `texture_count` and
`decoded_bytes` columns localise GPU- vs decode-side growth; re-walk
the table above for the responsible subsystem.

**Exit criterion (maintainer):** a 24 h soak flat before declaring W.2
closed, and the 7-day desktop run (below) started here and carried
through release.

### 1.5: everything on at once (rig, 2026-10-06)

1.5 added a lot that runs by itself, so before it shipped every piece
ran together: ten characters in the headless test rig (sway, software
rendering, a private D-Bus, no sound device), 30 minutes on native
Wayland and 20 on the X11 path. Two pairs played tag, the rest walked,
wandered, bounced or stood; greetings and bumping were on; a click
landed on a character every ten seconds (hearts, a line, its sound
decoded); three reminders ran — every minute, every two, one at a set
time — beside the focus timer at 1 + 1 minutes for the first quarter,
with desktop notifications going to a stand-in server; moods were on,
bedtime fell inside the run (dozing, yawns), one character kept hours
that brought it in and out (fading), and a stretch with no input sent
them to doze and brought a welcome back. The app's own emitter
(`ANIMA_SOAK_METRICS`, every 30 s) ran beside CPU, threads and file
descriptors read from `/proc`.

| | Native Wayland, 30 min | X11, 20 min |
|---|---|---|
| Resident memory | 229.8 → 225.3 MB | 193.3 → 195.2 MB (one step as they dozed off, flat after) |
| Textures, decoded bytes | 10, 4.46 MB — constant | the same |
| Threads, file descriptors | 142, 45 — constant | 144, 29 — constant |
| Log | 9 KB in 30 min, tag and greeting debug lines included | 11 KB in 20 min, the same |
| CPU, one core = 100 % | about 100 % while they move, 2 % dozing | about 470 %, 2 % dozing |
| Frame p95 | — | 1.4–3.8 ms |

The CPU is the rig's, not the app's: it renders in software — GLES on
llvmpipe for Wayland, lavapipe for X11, the only driver that gives the
X11 path a transparent window under XWayland — 60 frames a second
while anything moves. Most of the 142 threads are llvmpipe's and the
image decoders' pool, one per core of the 32; their number never moved.

Nothing grew. The run found four things wrong, all fixed before the X11
half: a reminder due during the focus timer's work went to the desktop's
notifications instead of waiting; the line for a turn of the timer
covered a reminder said at the same moment; a game of tag stopped for
good once a character held to the ground was "it" against one that
flies — two catches in 30 minutes, against 153 in the 20 after the fix;
and greetings remembered every pair that ever met. The sounds were only
decoded: with no device in the rig, playing them belongs to a desktop
run.

What 1.5 keeps in memory, and what bounds it:

| State | Bound |
|---|---|
| Little effects | 96 at most, the spent ones dropped every frame (`effects.rs`) |
| Greetings | the pairs near each other now; a pair's last greeting forgotten after the two-minute pause (`greet.rs`, fixed in this run) |
| Games of tag | one per pair playing, rebuilt every tick (`tag.rs`) |
| Reminder timers | one per reminder, and for one at a set time the day it was said (`reminders.rs`) |
| Desktop notifications | a queue of 16; past that they are dropped (`notify.rs`) |
| Lines for screen readers | the latest only (`announce.rs`) |
| Undo | 100 steps (`undo.rs`) |
| Sounds | decoded ones cached up to 32 MB, 8 started a tick at most (`audio.rs`) |
| Yawns | one per character, for that night (`moods.rs`) |
| Scene pictures | one texture per saved scene, replaced when its file changes |

### 1.5 again, after the loops learned to sleep (rig, 2026-10-12)

Late in 1.5 both loops changed how they wait: native Wayland sleeps
until its next deadline and is woken through a socket (R58), X11 waits
out 16 ms between frames, the X11 global shortcuts are grabbed by the
app, and reminders and the focus timer wake the loops on time. So the
everything-on run went again, 30 minutes on each backend: ten
characters — two pairs playing tag, one following another, walking,
wandering, bouncing, one keeping hours that began and ended in the run —
greetings, bumping and moods on, a tap every ten seconds, reminders
every one and two minutes, the focus timer at 1 + 1 for the first eight
minutes, bedtime from the 12th to the 17th, five minutes with no input
from the 20th (dozing, then the welcome back), the overlay hidden and
shown every ten minutes, a one-minute snooze, and every two minutes a
D-Bus `SetMuted` timed until it was saved. A row every 30 s from `/proc`.

| | Native Wayland, 30 min | X11, 30 min |
|---|---|---|
| Resident memory | 235.5 → 235.8 MB; second half flat | 193.4 → 194.1 MB; second half flat |
| Threads, file descriptors | 75, 41–42 | 77, 22 |
| D-Bus `SetMuted` saved after | 31–76 ms, every time | 9–77 ms, every time |
| Main thread, still (hidden, snoozed, asleep) | 2 wake-ups a second | 6 |
| Reminders said | 16 | 23 |
| Log | 41 KB | 36 KB, 12 greetings among it |

Nothing grew, nothing stopped answering. Wayland dozed and woke twice,
for bedtime and for the quiet stretch; X11 has no idle source in the
rig, so it dozed only for bedtime, which it does not log. The few
`ERROR` lines on Wayland are wgpu probing Vulkan on the rig's headless
outputs before it settles on GL, at start only.

### 1.5, two monitors at different scales (rig, 2026-10-13)

After native Wayland learned to draw each surface at its output's scale,
toasts to show outside edit mode, and each overlay to draw only when
what it shows changes, the same everything-on run went once more, on
two outputs side by side — one at scale 1, one at scale 2 — with two
more characters on the second (one wandering, one talking every
minute), and the scene paused and resumed from D-Bus every five
minutes, a toast each time, outside edit mode.

| | Native Wayland, 30 min | X11, 30 min |
|---|---|---|
| Resident memory | 266.7 → 267.3 MB; second half 267.2–267.3 | 196.2 → 195.6 MB; flat |
| Threads, file descriptors | 75, 45–46 | 79, 23 |
| D-Bus `SetMuted` saved after | 31–76 ms, all 14 times | 9–78 ms, all 15 times |
| Quiet stretch (no input, dozing) | 1–2% of a core, 3–4 main-thread wake-ups a second | no idle source in the rig |
| Reminders said, pauses | 17, 6 | 23, 5 |
| Log | 45 KB | 39 KB |

Memory a few tens of MB above the one-output run, for the second
output's surface and its buffer at twice the pixels; nothing grew.
Moving, the scene took a median 150% (Wayland) and 119% (X11) of a core
in the rig's software renderer across both outputs. X11's nine warnings
are all at start (no XSETTINGS, no portal or tray on the private bus,
XRandR's 0 mm screens, no validation layer, egui's note on the sRGB
framebuffer); Wayland's eight were not kept — its log was overwritten
by the X11 run — but no panic was counted on either.

## 7-day desktop protocol (manual, pre-1.0)

The automated soak runs headless on synthetic load; the real-session
soak catches what only a live desktop surfaces. Run once before 1.0:

1. Start animaEngine in your normal session with a representative
   scene (≥ 8 entities, ≥ 1 video, ≥ 1 GIF, behaviours on, perf
   overlay open). Leave it running for 7 days.
2. Each day, note RSS (`grep VmRSS /proc/$(pidof anima_engine)/status`)
   and the perf overlay's decoded-bytes / texture / p95 readings.
3. During the week, exercise: suspend/resume cycles (≥ 2), a monitor
   hotplug, a locale switch, a theme switch, several drag-drops and
   deletions, an edit-mode session per day.
4. Pass criteria: RSS flat after the first hour's warm-up (day-7 RSS
   within ~5% of day-1's post-warm-up reading); no growth in texture
   count or decoded bytes with a steady scene; no crash report left
   in `~/.cache/animaengine/crashes/`.

Record the run (dates, machine, compositor, day-by-day numbers) in the
release notes' soak section.
