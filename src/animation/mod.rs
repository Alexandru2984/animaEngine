pub mod cache;
pub mod frame;
pub mod gif_loader;
pub mod loader;
pub mod png_sequence;
pub mod spritesheet;
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
    pub fn tick(&mut self) -> bool {
        if !self.playing || self.frames.len() <= 1 {
            return false;
        }

        let frame_duration = self.current_frame_duration();
        let elapsed = self.last_frame_time.elapsed();

        if elapsed >= frame_duration {
            // Advance frame(s) — handles cases where multiple frames should be skipped
            let frames_to_advance = (elapsed.as_secs_f32() / frame_duration.as_secs_f32()) as usize;
            let frames_to_advance = frames_to_advance.max(1);

            self.current_frame = (self.current_frame + frames_to_advance) % self.frames.len();
            // Accumulate instead of resetting: preserves fractional time
            self.last_frame_time += frame_duration * frames_to_advance as u32;
            return true;
        }

        false
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
