<!--
Publishing checklist:
  - Post the markdown to your blog / dev.to / hashnode (dev.to has the
    best zero-setup reach for Rust content).
  - Submit to HN as "Show HN: animaEngine – ..." OR as a plain link to
    the post (plain link performs better for war-story posts; Show HN
    is for the project itself — pick ONE, don't double-submit same day).
  - Cross-post to r/rust (flair: "project") and lobste.rs (tag:
    rust, performance). r/rust mods like a comment from the author
    summarizing the technical core.
  - Best posting window: Tue–Thu, 14:00–16:00 UTC.
-->

# My desktop overlay was redrawing 60 times a second to show a static ghost

I'm building [animaEngine](https://github.com/Alexandru2984/animaEngine),
a Linux desktop overlay engine in Rust — think desktop pets: animated
characters that live on top of your desktop in a transparent,
click-through, always-on-top window, rendered with wgpu. It's the kind
of app that runs from login to shutdown, which means its idle behavior
matters more than its peak behavior.

Last week I profiled what it does when nothing happens. The answer was
embarrassing: a full render pass, 60 times a second, around the clock,
to draw an unchanged scene. A sleeping ghost sprite at 8 fps was
costing the GPU 60 wake-ups per second — and your laptop battery paid
for the difference.

## The classic winit loop, and why it's wrong for overlays

The standard winit render loop ends with an unconditional
`request_redraw()`:

```rust
// end of RedrawRequested handler:
if let Some(window) = &self.window {
    window.request_redraw();   // see you in 16 ms
}
```

With `PresentMode::Fifo`, `present()` blocks on vsync, so this
self-paces at display refresh. For a game, correct. For a desktop
overlay that spends 99% of its life with nothing moving, it means the
compositor, the GPU and your CPU governor never get to rest.

The numbers on my machine (Ubuntu, XWayland, integrated GPU): the
overlay sat at 2–3% CPU and kept the GPU permanently out of its
deepest idle state. Multiply by "runs 24/7" and it's the most
expensive feature nobody asked for.

## Deriving the frame rate from the scene

The fix is conceptually one sentence: **the next frame is due when
something will change, and the scene already knows when that is.**

Everything that changes on screen falls into one of three buckets:

```rust
pub(super) enum RedrawPacing {
    /// Something animates every tick (edit mode, toasts,
    /// autonomous behaviors, physics) — redraw at display refresh.
    Continuous,
    /// Scene is static except for playing sprite animations —
    /// sleep until the soonest next-frame deadline.
    Deadline(Instant),
    /// Nothing moves — sleep until the hot-reload heartbeat.
    Idle,
}
```

At the end of every frame, instead of `request_redraw()`, I compute
the pacing from live state. The interesting bucket is `Deadline`: each
animation already tracks when its current frame's hold expires, so the
soonest deadline across visible, playing sprites *is* the next frame
time:

```rust
pub fn next_frame_due(&self) -> Instant {
    self.last_frame_time + self.current_frame_duration()
}
```

An 8 fps sprite now wakes the loop 8 times a second instead of 60. A
paused scene wakes it 0.5 times a second (more on that heartbeat
below). Edit mode — where egui needs real interactivity — stays at
display refresh, as do toasts mid-fade and physics-driven entities.

The dispatch at the end of the redraw handler became:

```rust
match self.redraw_pacing() {
    RedrawPacing::Continuous => {
        event_loop.set_control_flow(ControlFlow::Wait);
        window.request_redraw();
    }
    RedrawPacing::Deadline(due) => {
        let heartbeat = Instant::now() + IDLE_HEARTBEAT;
        event_loop.set_control_flow(ControlFlow::WaitUntil(due.min(heartbeat)));
    }
    RedrawPacing::Idle => {
        event_loop.set_control_flow(
            ControlFlow::WaitUntil(Instant::now() + IDLE_HEARTBEAT));
    }
}
```

`WaitUntil` fires `StartCause::ResumeTimeReached` in `new_events`,
which requests the redraw. The chain re-arms itself every frame.

## The four gotchas

This is the part I wish someone had written up before me.

**1. Every input path must now wake the loop.** With a standing
`request_redraw()` you get redraws for free after any event. With
pacing, a click that arrives while the loop sleeps mutates state that
nothing will ever paint. Every input arm — mouse, keyboard, wheel,
drag-drop, tray events, and crucially *events egui consumed* — now
ends with a `request_redraw()`. winit coalesces them, so over-asking
is free; under-asking is a frozen UI.

**2. Hot-reload died when the scene went idle.** The config
hot-reload check (a 2 s mtime poll) lived in the redraw handler.
Fully static scene → no redraws → your config edits silently never
apply. That's why `Idle` isn't `Wait` forever: a 2-second heartbeat
keeps the poll alive at 0.5 wake-ups/s — still 120× fewer than
before, and the heartbeat does zero GPU work.

**3. Hidden windows may not get redraws at all.** When the overlay is
hidden from the tray, some compositors suppress `RedrawRequested`
delivery entirely. If your re-arm lives only in the redraw handler,
the chain breaks and never recovers. The fix: re-arm the heartbeat in
`new_events` (which timers always reach) *before* requesting the
redraw, and run the hot-reload check there too, so config changes
apply even while hidden.

**4. Don't trust the courtesy first frame.** Most platforms send an
initial `RedrawRequested` after window creation. The pacing chain
must not *depend* on that — one explicit `request_redraw()` at the
end of init costs nothing and removes a platform-specific landmine.

## Testing time-based code without sleeping

The frame-advance logic got property tests by adding one `#[cfg(test)]`
helper that rewinds the animation clock:

```rust
#[cfg(test)]
fn rewind(&mut self, by: Duration) {
    self.last_frame_time -= by;
}
```

Tests then assert exact frame positions after simulated stalls. One
lesson learned the flaky way: margins between the rewound clock and
the next assertion must dwarf scheduler noise — my first version used
50 ms margins and failed once under a parallel test run. Delays in the
tests are now tens of seconds; the suite still runs in milliseconds
because nothing actually sleeps.

While in there I found and fixed a real timing bug the old
divide-elapsed-by-current-duration frame skip had with variable
per-frame GIF delays — the walk now consumes each skipped frame's own
duration. Pacing work has a way of surfacing every assumption your
timing code ever made.

## Results

| Scene state | Wake-ups/s before | after |
|---|---|---|
| Static (paused / no animations) | 60 | 0.5 |
| One 8 fps sprite | 60 | 8 |
| Edit mode / toasts / physics | 60 | 60 |

Input latency is unchanged — events wake the loop immediately. The
native-Wayland path needed nothing: layer-shell surfaces are already
compositor-paced by frame callbacks, which is Wayland quietly having
solved this problem in the protocol design.

The whole change is ~140 lines including comments, in
[one commit](https://github.com/Alexandru2984/animaEngine/commit/2c4caa8).
If you maintain anything long-running on winit — overlays, widgets,
status bars, visualizers — check what your loop does at idle. Mine
was burning a watt to animate nothing.
