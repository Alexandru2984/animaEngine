pub mod frame;
pub mod gif_loader;
pub mod loader;
pub mod png_sequence;

use frame::Frame;
use std::time::Instant;

/// Animation state for a single entity.
/// Manages frame cycling, FPS timing, and play/pause.
#[derive(Debug)]
pub struct Animation {
    /// All frames for this animation
    pub frames: Vec<Frame>,
    /// Current frame index
    pub current_frame: usize,
    /// Frames per second
    pub fps: f32,
    /// Whether animation is playing
    pub playing: bool,
    /// Last time we advanced a frame
    last_frame_time: Instant,
}

impl Animation {
    pub fn new(frames: Vec<Frame>, fps: f32, playing: bool) -> Self {
        Self {
            frames,
            current_frame: 0,
            fps: fps.max(0.1), // Prevent division by zero
            playing,
            last_frame_time: Instant::now(),
        }
    }

    /// Advance the animation based on elapsed time.
    /// Returns true if the frame changed (texture needs update).
    pub fn tick(&mut self) -> bool {
        if !self.playing || self.frames.len() <= 1 {
            return false;
        }

        let frame_duration = std::time::Duration::from_secs_f32(1.0 / self.fps);
        let elapsed = self.last_frame_time.elapsed();

        if elapsed >= frame_duration {
            // Advance frame(s) — handles cases where multiple frames should be skipped
            let frames_to_advance = (elapsed.as_secs_f32() / frame_duration.as_secs_f32()) as usize;
            let frames_to_advance = frames_to_advance.max(1);

            self.current_frame = (self.current_frame + frames_to_advance) % self.frames.len();
            self.last_frame_time = Instant::now();
            return true;
        }

        false
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
