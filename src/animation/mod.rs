pub mod cache;
pub mod frame;
pub mod gif_loader;
pub mod loader;
pub mod png_sequence;
pub mod spritesheet;
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
    /// Uses per-frame delay if available, otherwise falls back to global FPS.
    fn current_frame_duration(&self) -> std::time::Duration {
        if let Some(frame) = self.frames.get(self.current_frame) {
            if let Some(delay_ms) = frame.delay_ms {
                if delay_ms > 0 {
                    return std::time::Duration::from_millis(delay_ms as u64);
                }
            }
        }
        // Fallback: use global FPS
        std::time::Duration::from_secs_f32(1.0 / self.fps)
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
}
