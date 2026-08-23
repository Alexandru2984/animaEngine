pub mod cache;
pub mod frame;
pub mod gif_loader;
pub mod loader;
pub mod png_sequence;
pub mod spritesheet;
#[cfg(feature = "video")]
pub mod video_loader;
pub mod webp_loader;

use frame::Frame;
use std::time::Instant;

/// Animation state for a single entity.
/// Manages frame cycling, FPS timing, and play/pause.
///
/// Supports two timing modes:
/// - **Global FPS**: All frames use the same duration (1/fps seconds)
/// - **Per-frame delays**: Each frame has its own delay from GIF/WebP metadata
#[derive(Debug)]
pub struct Animation {
    /// All frames for this animation
    pub frames: Vec<Frame>,
    /// Current frame index
    pub current_frame: usize,
    /// Frames per second (used when frames don't have individual delays)
    pub fps: f32,
    /// Whether animation is playing
    pub playing: bool,
    /// Last time we advanced a frame
    last_frame_time: Instant,
    /// Whether this animation uses per-frame delays (from GIF/WebP)
    pub has_per_frame_delays: bool,
    /// Optional easing curve applied to frame-interval timing. `None`
    /// means linear (the 0.2 behaviour). When set, the per-frame
    /// interval gets distorted so the loop's total duration stays
    /// exactly `n / fps` — see `crate::anim::EasingCurve::frame_interval`.
    /// Ignored when the asset carries per-frame delays (GIF / WebP),
    /// because those delays are authoritative.
    pub easing: Option<crate::anim::EasingCurve>,
}

impl Animation {
    pub fn new(frames: Vec<Frame>, fps: f32, playing: bool) -> Self {
        let has_per_frame_delays = frames.iter().any(|f| f.delay_ms.is_some());
        Self {
            frames,
            current_frame: 0,
            fps: fps.max(0.1), // Prevent division by zero
            playing,
            last_frame_time: Instant::now(),
            has_per_frame_delays,
            easing: None,
        }
    }

    /// Advance the animation based on elapsed time.
    /// Returns true if the frame changed (texture needs update).
    ///
    /// Skipped frames are walked one at a time, each consuming its
    /// *own* duration — per-frame GIF/WebP delays and easing-distorted
    /// intervals vary per frame, so dividing the whole elapsed span by
    /// the current frame's duration (the pre-0.5.5 behaviour) lands on
    /// the wrong frame whenever durations differ. The walk is bounded
    /// at two full loops; beyond that (system suspend, debugger pause)
    /// we resync the clock instead of replaying the backlog.
    pub fn tick(&mut self) -> bool {
        if !self.playing || self.frames.len() <= 1 {
            return false;
        }

        let mut advanced = false;
        let max_steps = self.frames.len() * 2;
        let mut steps = 0;

        loop {
            let frame_duration = self.current_frame_duration();
            if self.last_frame_time.elapsed() < frame_duration {
                break;
            }
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            // Accumulate instead of resetting: preserves fractional time.
            self.last_frame_time += frame_duration;
            advanced = true;
            steps += 1;
            if steps >= max_steps {
                self.last_frame_time = Instant::now();
                break;
            }
        }

        advanced
    }

    /// Pull `last_frame_time` into the past so tests can simulate
    /// elapsed wall-clock time without sleeping.
    #[cfg(test)]
    fn rewind(&mut self, by: std::time::Duration) {
        self.last_frame_time -= by;
    }

    /// Get the duration for the current frame.
    ///
    /// Priority:
    /// 1. Per-frame delay from GIF/WebP metadata when present
    ///    (authoritative — `Animation::easing` is ignored).
    /// 2. Otherwise: global FPS, optionally distorted by `easing` so
    ///    the loop's total duration stays at `n / fps` while the
    ///    individual frame holds vary.
    fn current_frame_duration(&self) -> std::time::Duration {
        if let Some(frame) = self.frames.get(self.current_frame) {
            if let Some(delay_ms) = frame.delay_ms {
                if delay_ms > 0 {
                    return std::time::Duration::from_millis(delay_ms as u64);
                }
            }
        }
        let baseline_total = self.frames.len() as f32 / self.fps;
        let interval = match self.easing {
            None | Some(crate::anim::EasingCurve::Linear) => 1.0 / self.fps,
            Some(curve) => {
                curve.frame_interval(self.current_frame, self.frames.len(), baseline_total)
            }
        };
        // Defensive: an extreme curve at very low frame count can in
        // theory produce a near-zero interval. Floor at 1ms so the
        // engine never tight-loops trying to advance the frame.
        let interval = interval.max(0.001);
        std::time::Duration::from_secs_f32(interval)
    }

    /// Get the current frame data
    pub fn current_frame_data(&self) -> Option<&Frame> {
        self.frames.get(self.current_frame)
    }

    /// When the current frame's hold expires — i.e. the earliest
    /// instant at which `tick()` would advance. Drives the idle-aware
    /// frame pacing in the render loop: a static scene sleeps until
    /// the soonest deadline across visible animations instead of
    /// redrawing at display refresh.
    pub fn next_frame_due(&self) -> Instant {
        self.last_frame_time + self.current_frame_duration()
    }

    /// Toggle play/pause
    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
        if self.playing {
            self.last_frame_time = Instant::now();
        }
    }

    /// Set FPS
    pub fn set_fps(&mut self, fps: f32) {
        self.fps = fps.max(0.1);
    }

    /// Number of frames
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Total decoded-RGBA bytes held by this animation. Used by the
    /// scene-level aggregate memory budget; saturates on overflow so a
    /// pathological corrupt frame can never wrap to a small number that
    /// would let the budget check pass spuriously.
    pub fn decoded_bytes(&self) -> usize {
        self.frames
            .iter()
            .map(|f| f.rgba.len())
            .fold(0usize, |acc, n| acc.saturating_add(n))
    }
}

/// Animation state an entity can be in (U.1). Closed enum — behavior
/// wiring matches exhaustively, so a new state can't ship half-wired.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum StateId {
    /// Default state; the only one guaranteed present.
    Idle,
    /// Horizontal locomotion (behavior-driven).
    Walk,
    /// Gravity-driven falling.
    Fall,
    /// While the user drags the entity.
    Drag,
}

/// A set of per-state animations with one active state (U.1).
///
/// Invariant: `Idle` is always present — constructors take the idle
/// animation by value. Lookups for missing states fall back to
/// `Idle`, so behavior wiring (U.2) can request any state without
/// caring whether the asset shipped it.
#[derive(Debug)]
pub struct AnimationSet {
    active: StateId,
    states: std::collections::BTreeMap<StateId, Animation>,
}

impl AnimationSet {
    /// Single-state set — exactly the pre-U.1 shape. Every legacy
    /// config and every plain drag-drop lands here.
    pub fn single(idle: Animation) -> Self {
        let mut states = std::collections::BTreeMap::new();
        states.insert(StateId::Idle, idle);
        Self {
            active: StateId::Idle,
            states,
        }
    }

    /// Insert/replace a state's animation. `Idle` may be replaced but
    /// never removed.
    pub fn insert(&mut self, state: StateId, animation: Animation) {
        self.states.insert(state, animation);
    }

    pub fn active_state(&self) -> StateId {
        self.active
    }

    /// `true` if the set carries a dedicated animation for `state`
    /// (no fallback considered).
    pub fn has_state(&self, state: StateId) -> bool {
        self.states.contains_key(&state)
    }

    /// The animation for the active state. Falls back to `Idle` when
    /// the active state has no dedicated sequence.
    pub fn current(&self) -> &Animation {
        self.states
            .get(&self.active)
            .or_else(|| self.states.get(&StateId::Idle))
            .expect("AnimationSet invariant: Idle always present")
    }

    pub fn current_mut(&mut self) -> &mut Animation {
        let key = if self.states.contains_key(&self.active) {
            self.active
        } else {
            StateId::Idle
        };
        self.states
            .get_mut(&key)
            .expect("AnimationSet invariant: Idle always present")
    }

    /// Switch the active state. Returns `true` when the *effective*
    /// animation changed (the caller marks the GPU texture dirty).
    /// Switching to a missing state falls back to Idle; switching to
    /// the state already active is a no-op. A real switch rewinds the
    /// target to frame 0 with a fresh clock so a revisited state
    /// doesn't resume mid-loop.
    pub fn switch(&mut self, to: StateId) -> bool {
        let effective_now = if self.states.contains_key(&self.active) {
            self.active
        } else {
            StateId::Idle
        };
        let effective_next = if self.states.contains_key(&to) {
            to
        } else {
            StateId::Idle
        };
        self.active = to;
        if effective_now == effective_next {
            return false;
        }
        let anim = self
            .states
            .get_mut(&effective_next)
            .expect("effective state exists by construction");
        anim.current_frame = 0;
        anim.last_frame_time = Instant::now();
        true
    }

    /// Total decoded bytes across **all** states — the memory budget
    /// must count inactive sequences too.
    pub fn decoded_bytes(&self) -> usize {
        self.states
            .values()
            .fold(0usize, |acc, a| acc.saturating_add(a.decoded_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// 1×1 frame carrying a per-frame delay, GIF-style.
    fn delayed_frame(delay_ms: u32) -> Frame {
        Frame::with_delay(vec![0u8; 4], 1, 1, delay_ms)
    }

    // Delays below are tens of seconds so the margin between the
    // rewound clock and the next frame boundary dwarfs any scheduling
    // stall the parallel test runner can introduce.

    #[test]
    fn tick_advances_one_frame_after_its_own_delay() {
        let mut anim = Animation::new(
            vec![delayed_frame(10_000), delayed_frame(50_000)],
            10.0,
            true,
        );
        anim.rewind(Duration::from_millis(15_000));
        assert!(anim.tick());
        // 10s consumed; remaining 5s < the 50s of frame 1.
        assert_eq!(anim.current_frame, 1);
    }

    #[test]
    fn tick_walks_variable_delays_not_current_duration() {
        // Delays 10 / 30 / 60 s. After 45s the walk consumes
        // 10 (→1) + 30 (→2) and stops: 5s < 60s.
        // The pre-0.5.5 division-based skip computed 45/10 = 4
        // steps → frame 1 — wrong frame *and* wrong clock.
        let mut anim = Animation::new(
            vec![
                delayed_frame(10_000),
                delayed_frame(30_000),
                delayed_frame(60_000),
            ],
            10.0,
            true,
        );
        anim.rewind(Duration::from_millis(45_000));
        assert!(anim.tick());
        assert_eq!(anim.current_frame, 2);
        // Clock was advanced by exactly 40s — immediate re-tick
        // must not advance again (5s of margin).
        assert!(!anim.tick());
        assert_eq!(anim.current_frame, 2);
    }

    #[test]
    fn tick_resyncs_after_long_stall_instead_of_replaying() {
        let mut anim = Animation::new(
            vec![
                delayed_frame(10_000),
                delayed_frame(10_000),
                delayed_frame(10_000),
            ],
            10.0,
            true,
        );
        // Simulate a ~17min suspend — 100 frames of backlog. The walk
        // must cap at two loops (6 steps) and resync, not replay 100.
        anim.rewind(Duration::from_secs(1_000));
        assert!(anim.tick());
        // After resync the clock is fresh: no re-advance for another
        // 10s — far beyond any test-runner stall.
        assert!(!anim.tick());
    }

    #[test]
    fn tick_does_not_advance_when_paused_or_single_frame() {
        let mut paused = Animation::new(vec![delayed_frame(10), delayed_frame(10)], 10.0, false);
        paused.rewind(Duration::from_secs(1));
        assert!(!paused.tick());

        let mut single = Animation::new(vec![delayed_frame(10)], 10.0, true);
        single.rewind(Duration::from_secs(1));
        assert!(!single.tick());
    }

    fn one_frame_anim(marker: u8) -> Animation {
        Animation::new(vec![Frame::new(vec![marker; 4], 1, 1)], 1.0, true)
    }

    fn two_frame_anim() -> Animation {
        Animation::new(
            vec![
                Frame::new(vec![0u8; 4], 1, 1),
                Frame::new(vec![1u8; 4], 1, 1),
            ],
            1.0,
            true,
        )
    }

    #[test]
    fn set_single_serves_idle_for_every_state() {
        let mut set = AnimationSet::single(one_frame_anim(7));
        assert_eq!(set.active_state(), StateId::Idle);
        // Switching to a missing state falls back to Idle — and is
        // NOT an effective change.
        assert!(!set.switch(StateId::Walk));
        assert_eq!(set.current().frames[0].rgba[0], 7);
    }

    #[test]
    fn set_switch_to_present_state_resets_frame_and_reports_change() {
        let mut set = AnimationSet::single(one_frame_anim(1));
        let mut walk = two_frame_anim();
        walk.current_frame = 1; // dirty runtime state, must reset
        set.insert(StateId::Walk, walk);

        assert!(set.switch(StateId::Walk));
        assert_eq!(set.active_state(), StateId::Walk);
        assert_eq!(set.current().current_frame, 0, "switch rewinds");
        // Same state again: no-op.
        assert!(!set.switch(StateId::Walk));
    }

    #[test]
    fn set_missing_to_missing_switch_is_noop() {
        let mut set = AnimationSet::single(one_frame_anim(1));
        assert!(!set.switch(StateId::Fall));
        assert!(!set.switch(StateId::Drag));
        // Falling back to Idle the whole time — and switching BACK to
        // idle from a fallback is also not a change.
        assert!(!set.switch(StateId::Idle));
    }

    #[test]
    fn set_decoded_bytes_counts_inactive_states() {
        let mut set = AnimationSet::single(one_frame_anim(1)); // 4 bytes
        set.insert(StateId::Walk, two_frame_anim()); // 8 bytes
        assert_eq!(set.decoded_bytes(), 12);
    }

    #[test]
    fn tick_fixed_fps_advances_by_elapsed_over_interval() {
        // No per-frame delays → global FPS path. 0.1 fps = 10s per
        // frame; 35s of backlog advances exactly 3 frames (5s margin).
        let frames: Vec<Frame> = (0..5).map(|_| Frame::new(vec![0u8; 4], 1, 1)).collect();
        let mut anim = Animation::new(frames, 0.1, true);
        anim.rewind(Duration::from_millis(35_000));
        assert!(anim.tick());
        assert_eq!(anim.current_frame, 3);
        assert!(!anim.tick());
    }
}
