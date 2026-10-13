//! Transient on-screen notifications.
//!
//! Toasts are short-lived messages (3-6 s) used to confirm user actions
//! (saved config, duplicated entity) or surface non-fatal failures
//! (asset too large, decode error). They're rendered as a stack in the
//! bottom-right corner, over the settings panel in edit mode and over
//! the desktop outside it — where they take no clicks: the overlay's
//! input region there is the ⚙ corner and the characters.

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
    /// Its number in the queue's life: a screen reader is told of a
    /// toast by a node it has not seen before (`announce`).
    pub serial: u64,
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

/// How long a toast takes to slide in (`panels::toasts`).
pub const SLIDE_IN: Duration = Duration::from_millis(200);
/// How long a toast takes to fade out, at the end of its lifetime.
pub const FADE_OUT: Duration = Duration::from_millis(300);

#[derive(Default)]
pub struct ToastQueue {
    toasts: Vec<Toast>,
    /// The next toast's `serial`.
    next_serial: u64,
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
        self.next_serial += 1;
        self.toasts.push(Toast {
            kind,
            message: msg.into(),
            created_at: Instant::now(),
            lifetime,
            serial: self.next_serial,
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

    /// When the stack next looks different: at once while a toast slides
    /// in or fades out, else when the next one starts to fade — with
    /// `reduced` motion, when the next one goes. `None` with none to
    /// show. Both loops wake for it, in either mode; the stack asked egui
    /// for a frame every frame while it showed, sixty a second for the
    /// three to six seconds of a toast.
    pub fn next_change(&self, now: Instant, reduced: bool) -> Option<Instant> {
        self.toasts
            .iter()
            .map(|t| {
                let ends = t.expires_at();
                if reduced {
                    return ends;
                }
                let fades = ends.checked_sub(FADE_OUT).unwrap_or(ends);
                if now < t.created_at + SLIDE_IN || now >= fades {
                    now
                } else {
                    fades
                }
            })
            .min()
    }
}

/// Tell a screen reader each toast, once: a live node of its own while it
/// lasts, as a character's line is (`crate::ui::speech::announce`) —
/// politely for news, at once for a warning or an error — in both modes,
/// as the toasts are drawn in both. Nothing with the bridge off.
pub fn announce(ctx: &egui::Context, queue: &ToastQueue) {
    for toast in queue.iter() {
        ctx.accesskit_node_builder(egui::Id::new(("anima.toast", toast.serial)), |node| {
            node.set_role(egui::accesskit::Role::Label);
            node.set_value(toast.message.clone());
            node.set_live(match toast.kind {
                ToastKind::Warn | ToastKind::Error => egui::accesskit::Live::Assertive,
                ToastKind::Info | ToastKind::Success => egui::accesskit::Live::Polite,
            });
        });
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

    /// Each toast is a node of its own to a screen reader, even when the
    /// words are the same as the last one's.
    #[test]
    fn each_toast_has_a_number_of_its_own() {
        let mut q = ToastQueue::default();
        q.warn("Paste failed");
        q.warn("Paste failed");
        let serials: Vec<u64> = q.iter().map(|t| t.serial).collect();
        assert_eq!(serials.len(), 2);
        assert_ne!(serials[0], serials[1]);
    }

    /// Frames while a toast moves, none while it sits still: it slides
    /// in, waits, and fades out; with reduced motion it only goes.
    #[test]
    fn a_toast_wants_frames_only_while_it_moves() {
        let mut q = ToastQueue::default();
        let now = Instant::now();
        assert_eq!(q.next_change(now, false), None);
        q.info("Saved");
        let t = q.iter().next().unwrap().clone();
        let fades = t.expires_at() - FADE_OUT;
        assert_eq!(q.next_change(t.created_at, false), Some(t.created_at));
        let still = t.created_at + SLIDE_IN + Duration::from_millis(1);
        assert_eq!(q.next_change(still, false), Some(fades));
        assert_eq!(q.next_change(fades, false), Some(fades));
        assert_eq!(q.next_change(t.created_at, true), Some(t.expires_at()));
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
            serial: 0,
        });
        q.info("fresh");
        q.prune();

        let remaining: Vec<&str> = q.iter().map(|t| t.message.as_str()).collect();
        assert_eq!(remaining, vec!["fresh"]);
    }
}
