// The Vulkan driver of a laptop's NVIDIA GPU, left unloaded where an
// integrated GPU draws: Linux's DRM devices tell which there are.
#[cfg(target_os = "linux")]
pub mod hybrid;
pub mod sprite;

/// A render error said once, not every frame (1.5). A surface that kept
/// failing failed sixty times a second, and each was a warning line: a
/// log growing by megabytes an hour, in whatever keeps the app's output.
/// The first of a kind is said; its repeats are counted, and said once
/// frames draw again or another error comes.
#[derive(Debug, Default)]
pub struct ErrorOnce {
    last: Option<String>,
    repeats: u64,
}

impl ErrorOnce {
    /// A frame on `surface` failed with `error`.
    pub fn failed(&mut self, surface: &str, error: &impl std::fmt::Debug) {
        let said = format!("{surface}: {error:?}");
        if self.last.as_deref() == Some(said.as_str()) {
            self.repeats += 1;
            return;
        }
        self.end_streak();
        tracing::warn!("Render error on {said}");
        self.last = Some(said);
    }

    /// A frame drew.
    pub fn drew(&mut self) {
        self.end_streak();
    }

    fn end_streak(&mut self) {
        if let Some(last) = self.last.take() {
            if self.repeats > 0 {
                tracing::warn!("Render error on {last}: {} more times", self.repeats);
            }
        }
        self.repeats = 0;
    }
}
pub mod texture;
pub mod wgpu_renderer;
// Windows presents through UpdateLayeredWindow rather than a swapchain —
// see the module docs for why a swapchain can't be transparent there.
#[cfg(windows)]
pub mod win_layered;

#[cfg(test)]
mod tests {
    use super::ErrorOnce;

    #[test]
    fn a_render_error_is_said_once_and_its_repeats_counted() {
        let mut errors = ErrorOnce::default();
        for _ in 0..60 {
            errors.failed("the overlay", &"Other");
        }
        assert_eq!(errors.repeats, 59, "said once, the rest counted");
        errors.failed("the overlay", &"Outdated");
        assert_eq!(errors.repeats, 0, "another error is said, a new streak");
        errors.drew();
        assert!(errors.last.is_none() && errors.repeats == 0);
    }
}
