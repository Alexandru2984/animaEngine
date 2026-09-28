//! Config hot-reload on the winit path.
//!
//! The watcher and what to do with its result are shared with the native
//! Wayland loop (`crate::config_watch`); this only feeds them `App`'s
//! fields once a frame.

use super::App;

impl App {
    /// Collect a finished reload, or start one if the file changed.
    #[tracing::instrument(skip(self))]
    pub(super) fn check_hot_reload(&mut self) {
        let poll = self.config_watch.poll(self.config_dirty);
        let reloaded = matches!(poll, crate::config_watch::Poll::Ready(_));
        crate::config_watch::handle(
            poll,
            &mut outcome_ctx!(self),
            &mut self.config,
            &mut self.warnings,
        );
        if reloaded {
            // The scene was replaced from the file: the steps no longer
            // describe it.
            self.history.clear();
        }
    }
}
