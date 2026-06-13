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
   in `~/.cache/animaEngine/crashes/`.

Record the run (dates, machine, compositor, day-by-day numbers) in the
release notes' soak section.
