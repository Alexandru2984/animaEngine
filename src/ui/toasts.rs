//! Transient on-screen notifications.
//!
//! Toasts are short-lived messages (3-6 s) used to confirm user actions
//! (saved config, duplicated entity) or surface non-fatal failures
//! (asset too large, decode error). They're rendered as a stack in the
//! bottom-right corner above the settings panel.

use std::time::{Duration, Instant};

/// Severity classes — drive the background color and default lifetime.
#[derive(Clone, Copy, Debug)]
pub enum ToastKind {
    Info,
    Success,
    Warn,
    Error,
}

#[derive(Clone)]
pub struct Toast {
    pub kind: ToastKind,
    pub message: String,
    /// When the toast was queued. Drives the slide-in / fade-out timing
    /// in the renderer (`panels::toast_card`); `prune` uses
    /// `expires_at()` for retention.
    pub created_at: Instant,
    /// Total time the toast remains visible before `prune` removes it.
    pub lifetime: Duration,
}

impl Toast {
    /// Absolute removal time. Derived rather than stored so we can't
    /// hand out a struct whose `created_at + lifetime` disagrees with
    /// `expires_at`.
    pub fn expires_at(&self) -> Instant {
        self.created_at + self.lifetime
    }

    /// How long this toast has been on screen, clamped at 0 so a system
    /// clock step backwards can't produce a negative.
    pub fn age(&self) -> Duration {
        Instant::now().saturating_duration_since(self.created_at)
    }

    /// How much longer the toast has before it auto-expires.
    pub fn remaining(&self) -> Duration {
        self.expires_at().saturating_duration_since(Instant::now())
    }
}

/// Hard cap so a tight loop emitting toasts can't pin them all visible.
const MAX_TOASTS: usize = 8;

#[derive(Default)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
}

impl ToastQueue {
    pub fn info(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Info, msg, Duration::from_secs(3));
    }
    pub fn success(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Success, msg, Duration::from_secs(3));
    }
    pub fn warn(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Warn, msg, Duration::from_secs(5));
    }
    pub fn error(&mut self, msg: impl Into<String>) {
        self.push(ToastKind::Error, msg, Duration::from_secs(6));
    }

    fn push(&mut self, kind: ToastKind, msg: impl Into<String>, lifetime: Duration) {
        self.toasts.push(Toast {
            kind,
            message: msg.into(),
            created_at: Instant::now(),
            lifetime,
        });
        if self.toasts.len() > MAX_TOASTS {
            // Drop the oldest — newer messages are usually more relevant.
            self.toasts.remove(0);
        }
    }

    /// Remove expired toasts. Cheap (O(n) with n ≤ 8) — call once per frame
    /// before rendering.
    pub fn prune(&mut self) {
        let now = Instant::now();
        self.toasts.retain(|t| t.expires_at() > now);
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Toast> {
        self.toasts.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_drops_oldest() {
        let mut q = ToastQueue::default();
        for i in 0..(MAX_TOASTS + 3) {
            q.info(format!("msg {i}"));
        }
        assert_eq!(q.iter().count(), MAX_TOASTS);
        // The first three pushes must have been evicted.
        let messages: Vec<&str> = q.iter().map(|t| t.message.as_str()).collect();
        assert!(!messages.contains(&"msg 0"));
        assert!(messages.contains(&"msg 3"));
    }

    #[test]
    fn prune_removes_expired() {
        let mut q = ToastQueue::default();
        // Synthesize one already-expired toast plus one fresh.
        q.toasts.push(Toast {
            kind: ToastKind::Info,
            message: "old".into(),
            created_at: Instant::now() - Duration::from_secs(10),
            lifetime: Duration::from_secs(1),
        });
        q.info("fresh");
        q.prune();

        let remaining: Vec<&str> = q.iter().map(|t| t.message.as_str()).collect();
        assert_eq!(remaining, vec!["fresh"]);
    }
}
