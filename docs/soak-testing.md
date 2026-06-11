# Soak & resilience testing

Started in 0.6 (T.10) with the suspend/resume + stall protocol; the
long-run soak harness lands in 0.9 (W.1) and extends this file.

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
