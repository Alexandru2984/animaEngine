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
    pub expires_at: Instant,
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
            expires_at: Instant::now() + lifetime,
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
        self.toasts.retain(|t| t.expires_at > now);
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
            expires_at: Instant::now() - Duration::from_secs(1),
        });
        q.info("fresh");
        q.prune();

        let remaining: Vec<&str> = q.iter().map(|t| t.message.as_str()).collect();
        assert_eq!(remaining, vec!["fresh"]);
    }
}
