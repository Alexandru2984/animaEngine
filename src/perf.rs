//! Frame-time + per-system perf sampling (D.6).
//!
//! Owned by `App`, ticked once per frame around the render loop.
//! The overlay widget in `ui::perf_overlay` reads the sampler each
//! frame; the snapshot exporter walks the ring buffer and writes a
//! chrome-tracing compatible JSON to `~/.cache/animaEngine/`.
//!
//! Design notes:
//!
//! - All timing uses [`std::time::Instant`] — no profiling-crate
//!   dependency, no platform-specific paths.
//! - The history is a fixed-size ring buffer (1024 frames ≈ 17 s at
//!   60 fps; at 1000 fps it still covers a full second). The 60-frame
//!   short window drives the on-screen averages; the full ring backs
//!   the snapshot export.
//! - `Category` is `Copy + Hash`; the per-frame `EnumMap`-style array
//!   keeps lookups O(1) without an actual HashMap. There are five
//!   buckets, never more, so `[Duration; N]` is the right shape.
//! - The sampler is allocation-free in steady state — the ring buffer
//!   is preallocated; per-frame data updates fixed slots.
//!
//! The whole module is non-`#[cfg(debug_assertions)]`-gated so the
//! data structures stay testable. The overlay's *visibility* and the
//! keybinding to toggle it are gated separately (see
//! `App::dispatch_action`), so a release build that never enables
//! the overlay also never executes the timing scopes — they're
//! short-circuited at the call site, not the type level.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Coarse-grained timing buckets covering one frame. Order in the
/// declaration is the display order in the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Category {
    /// Scene update — entity animation step, behaviour tick,
    /// physics integration, group/visibility resolve.
    SceneUpdate,
    /// egui pass — `ctx.run` + layout + paint.
    EguiPaint,
    /// wgpu submission — buffer uploads + draw + queue submit.
    WgpuSubmit,
    /// Present + swap-chain acquire.
    Present,
    /// Whatever's left in the frame — vsync wait, OS scheduling,
    /// hot-reload check, tray crank, etc.
    Idle,
}

impl Category {
    pub const ALL: &'static [Self] = &[
        Self::SceneUpdate,
        Self::EguiPaint,
        Self::WgpuSubmit,
        Self::Present,
        Self::Idle,
    ];

    /// Stable name surfaced in the overlay UI and in the snapshot JSON
    /// (used as the chrome-tracing `cat` field).
    pub fn label(self) -> &'static str {
        match self {
            Self::SceneUpdate => "scene_update",
            Self::EguiPaint => "egui_paint",
            Self::WgpuSubmit => "wgpu_submit",
            Self::Present => "present",
            Self::Idle => "idle",
        }
    }
}

/// One frame's worth of timing data — total + per-category buckets.
/// Stored in the ring buffer; the snapshot exporter walks the
/// history producing one chrome-tracing event per category per
/// frame.
#[derive(Debug, Clone, Copy)]
pub struct FrameSample {
    /// Real-time start of the frame, used as the chrome-tracing `ts`
    /// anchor. Relative to the sampler's `epoch` so very long
    /// sessions don't overflow `u64` microseconds.
    pub frame_start: Duration,
    /// Total frame time (frame_start_next - frame_start_this) —
    /// the canonical "fps" denominator.
    pub total: Duration,
    /// Per-category accumulated duration. Always sums to ≤ `total`;
    /// difference is the implicit "Idle" bucket.
    pub by_category: [Duration; 5],
}

impl FrameSample {
    fn empty(frame_start: Duration) -> Self {
        Self {
            frame_start,
            total: Duration::ZERO,
            by_category: [Duration::ZERO; 5],
        }
    }
}

/// Ring-buffer history of [`FrameSample`]s + the in-flight frame's
/// scratch state. `App` owns one, drives it by calling
/// `begin_frame` → `scope(...)` per work item → `end_frame` once per
/// rendered frame.
pub struct PerfSampler {
    /// Sampler creation time — every `frame_start` is relative to
    /// this so the chrome-tracing JSON stays compact.
    epoch: Instant,
    /// Capacity of `history`. Fixed at 1024 — at 60 fps that's ~17 s
    /// of context; the snapshot export takes the full window.
    capacity: usize,
    history: VecDeque<FrameSample>,
    /// In-progress frame: filled by `scope`s, finalised by
    /// `end_frame`.
    current: FrameSample,
    /// Real-time start of the in-progress frame, captured at
    /// `begin_frame`. Used to compute `current.total` at
    /// `end_frame`.
    current_start_instant: Option<Instant>,
}

impl Default for PerfSampler {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl PerfSampler {
    pub fn new(capacity: usize) -> Self {
        Self {
            epoch: Instant::now(),
            capacity,
            history: VecDeque::with_capacity(capacity),
            current: FrameSample::empty(Duration::ZERO),
            current_start_instant: None,
        }
    }

    /// Begin a new frame's worth of measurements. Idempotent — called
    /// at the top of every render iteration regardless of overlay
    /// visibility; the cost is one `Instant::now()` and an array
    /// reset, which is below frame-time noise even at 1000 fps.
    pub fn begin_frame(&mut self) {
        let now = Instant::now();
        self.current_start_instant = Some(now);
        self.current = FrameSample::empty(now.saturating_duration_since(self.epoch));
    }

    /// Record `dur` against `cat`. Multiple scopes against the same
    /// category sum — useful for split-render-pass scenarios.
    pub fn add(&mut self, cat: Category, dur: Duration) {
        let i = cat as usize;
        self.current.by_category[i] = self.current.by_category[i].saturating_add(dur);
    }

    /// Begin a scoped measurement returning a `Scope` guard. Dropping
    /// the guard adds the elapsed time to the given category. Use
    /// inside a small block; do *not* hold across `.await`s — the
    /// sampler is single-threaded by design.
    pub fn scope(&mut self, cat: Category) -> Scope<'_> {
        Scope {
            sampler: self,
            cat,
            start: Instant::now(),
        }
    }

    /// Finalise the current frame: compute `total`, push into the
    /// ring buffer, evict the oldest if we're at capacity.
    pub fn end_frame(&mut self) {
        let Some(start) = self.current_start_instant.take() else {
            return;
        };
        self.current.total = start.elapsed();
        if self.history.len() == self.capacity {
            self.history.pop_front();
        }
        self.history.push_back(self.current);
    }

    /// Average frame total over the last `n` frames (capped to
    /// history length). Returns `None` when history is empty so the
    /// overlay can render a "warming up" placeholder.
    pub fn recent_avg_total(&self, n: usize) -> Option<Duration> {
        let take = self.history.len().min(n);
        if take == 0 {
            return None;
        }
        let sum: Duration = self.history.iter().rev().take(take).map(|s| s.total).sum();
        Some(sum / take as u32)
    }

    /// p95 frame total over the last `n` frames. Useful for catching
    /// stutters that average-fps hides.
    pub fn recent_p95_total(&self, n: usize) -> Option<Duration> {
        let take = self.history.len().min(n);
        if take == 0 {
            return None;
        }
        let mut window: Vec<Duration> = self
            .history
            .iter()
            .rev()
            .take(take)
            .map(|s| s.total)
            .collect();
        window.sort_unstable();
        let idx = ((take as f32) * 0.95) as usize;
        Some(window[idx.min(take - 1)])
    }

    /// Average for one category over the last `n` frames.
    pub fn recent_avg_category(&self, cat: Category, n: usize) -> Option<Duration> {
        let take = self.history.len().min(n);
        if take == 0 {
            return None;
        }
        let i = cat as usize;
        let sum: Duration = self
            .history
            .iter()
            .rev()
            .take(take)
            .map(|s| s.by_category[i])
            .sum();
        Some(sum / take as u32)
    }

    /// Snapshot view of the full history for export. Returns the
    /// concrete `VecDeque::Iter` rather than an `impl Iterator` so
    /// callers can take advantage of `DoubleEndedIterator` (needed
    /// by the overlay's sparkline that walks newest-first).
    pub fn history(&self) -> std::collections::vec_deque::Iter<'_, FrameSample> {
        self.history.iter()
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn epoch(&self) -> Instant {
        self.epoch
    }
}

/// Read resident-set size (in KiB) from `/proc/self/status`.
/// Linux-only — returns `None` on other platforms or when the proc
/// file isn't readable. Cheap (sub-microsecond) but called only
/// every N frames so even the syscall is invisible.
pub fn read_rss_kib() -> Option<u64> {
    let content = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let value: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(value);
        }
    }
    None
}

/// Write a chrome-tracing compatible JSON snapshot of the current
/// history at `~/.cache/animaEngine/perf-<unix-ts>.json`. Returns the
/// path on success. Format spec:
/// https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU
///
/// One event per category per frame; `ts` and `dur` are microseconds
/// since the sampler's epoch so durations align across rows. The JSON
/// is hand-written to avoid pulling serde_json just for this export
/// surface — the schema is tiny and stable.
pub fn export_snapshot(sampler: &PerfSampler) -> std::io::Result<PathBuf> {
    let dir = cache_dir();
    let ts_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("perf-{ts_nanos}.json"));
    export_snapshot_to(sampler, &path)
}

/// Same as `export_snapshot` but writes to the supplied path. Tests
/// route through this with a tempdir to avoid colliding on a shared
/// `~/.cache` path; production code uses the no-arg variant above.
pub fn export_snapshot_to(sampler: &PerfSampler, path: &Path) -> std::io::Result<PathBuf> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut out = String::with_capacity(8 * sampler.history_len());
    out.push_str(r#"{"traceEvents":["#);
    let mut first = true;
    for sample in sampler.history() {
        for &cat in Category::ALL {
            let dur = sample.by_category[cat as usize].as_micros();
            if dur == 0 {
                continue;
            }
            if !first {
                out.push(',');
            }
            first = false;
            // Per-category events anchor at the frame_start; consecutive
            // categories overlap visually in the trace UI but that's
            // accurate — they're slices of one wall-clock frame.
            let ts = sample.frame_start.as_micros();
            out.push_str(&format!(
                r#"{{"name":"{}","cat":"frame","ph":"X","ts":{},"dur":{},"pid":1,"tid":1}}"#,
                cat.label(),
                ts,
                dur,
            ));
        }
        // Total frame as a separate event so the trace shows the
        // outer envelope around the per-category slices.
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&format!(
            r#"{{"name":"frame","cat":"frame","ph":"X","ts":{},"dur":{},"pid":1,"tid":0}}"#,
            sample.frame_start.as_micros(),
            sample.total.as_micros(),
        ));
    }
    out.push_str(r#"],"displayTimeUnit":"ms"}"#);

    crate::util::atomic_write_bytes(path, out.as_bytes())
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(path.to_path_buf())
}

fn cache_dir() -> PathBuf {
    if let Some(proj) = directories::ProjectDirs::from("", "", "animaEngine") {
        return proj.cache_dir().to_path_buf();
    }
    // M3 hardening (0.5.2): the previous fallback was
    // `$HOME/.cache/animaEngine` with `HOME` defaulting to `.` when
    // unset, which let a wrapper script `HOME=/etc/cron.d anima-engine`
    // direct cache writes anywhere. Prefer an absolute, uid-scoped
    // tmpdir so the fallback path always lands inside `/tmp` no
    // matter the env. `std::env::temp_dir` honours `TMPDIR` but
    // still resolves to an absolute path; combined with the uid
    // suffix two users on a shared host don't collide.
    // SAFETY: libc::getuid is a POSIX syscall with no preconditions
    // and no failure modes; it returns the calling process's real UID
    // and has no FFI safety obligations.
    let uid = unsafe { libc::getuid() };
    std::env::temp_dir().join(format!("animaEngine-{uid}"))
}

/// RAII guard that adds its elapsed lifetime to the sampler under
/// the chosen category. Returned by [`PerfSampler::scope`].
pub struct Scope<'s> {
    sampler: &'s mut PerfSampler,
    cat: Category,
    start: Instant,
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        self.sampler.add(self.cat, elapsed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sampler_reports_no_averages() {
        let s = PerfSampler::new(8);
        assert!(s.recent_avg_total(10).is_none());
        assert!(s.recent_p95_total(10).is_none());
        for &c in Category::ALL {
            assert!(s.recent_avg_category(c, 10).is_none());
        }
    }

    #[test]
    fn frame_lifecycle_populates_history() {
        let mut s = PerfSampler::new(4);
        for _ in 0..3 {
            s.begin_frame();
            s.add(Category::SceneUpdate, Duration::from_micros(500));
            s.add(Category::EguiPaint, Duration::from_micros(800));
            s.end_frame();
        }
        assert_eq!(s.history_len(), 3);
        // Per-category averages reflect what we added.
        let avg_scene = s.recent_avg_category(Category::SceneUpdate, 10).unwrap();
        assert_eq!(avg_scene, Duration::from_micros(500));
    }

    #[test]
    fn ring_buffer_evicts_oldest() {
        let mut s = PerfSampler::new(2);
        for _ in 0..5 {
            s.begin_frame();
            s.end_frame();
        }
        // Capacity 2, 5 pushes → length stays at 2 (oldest evicted).
        assert_eq!(s.history_len(), 2);
    }

    #[test]
    fn scope_guard_adds_on_drop() {
        let mut s = PerfSampler::new(4);
        s.begin_frame();
        {
            let _scope = s.scope(Category::SceneUpdate);
            std::thread::sleep(Duration::from_millis(2));
        }
        s.end_frame();
        let avg = s.recent_avg_category(Category::SceneUpdate, 1).unwrap();
        assert!(
            avg >= Duration::from_millis(1),
            "expected >= 1ms, got {avg:?}"
        );
    }

    #[test]
    fn end_frame_without_begin_is_noop() {
        let mut s = PerfSampler::new(4);
        s.end_frame(); // no panic; no history append
        assert_eq!(s.history_len(), 0);
    }

    #[test]
    fn export_writes_valid_json_with_at_least_one_event() {
        let mut s = PerfSampler::new(4);
        for _ in 0..3 {
            s.begin_frame();
            s.add(Category::SceneUpdate, Duration::from_micros(100));
            s.end_frame();
        }
        // Use a per-test temp path so two `cargo test` workers can run
        // export tests in parallel without colliding.
        let path =
            std::env::temp_dir().join(format!("anima-perf-test-{}.json", std::process::id()));
        export_snapshot_to(&s, &path).expect("export ok");
        let content = std::fs::read_to_string(&path).expect("file readable");
        assert!(content.starts_with(r#"{"traceEvents":["#));
        assert!(content.contains(r#""name":"scene_update""#));
        assert!(content.contains(r#""name":"frame""#));
        assert!(content.ends_with(r#"],"displayTimeUnit":"ms"}"#));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_with_empty_history_still_produces_valid_envelope() {
        let s = PerfSampler::new(4);
        let path =
            std::env::temp_dir().join(format!("anima-perf-test-empty-{}.json", std::process::id()));
        export_snapshot_to(&s, &path).expect("export ok");
        let content = std::fs::read_to_string(&path).expect("file readable");
        assert_eq!(content, r#"{"traceEvents":[],"displayTimeUnit":"ms"}"#);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn p95_returns_top_quantile() {
        let mut s = PerfSampler::new(100);
        for i in 0..100 {
            s.begin_frame();
            // Synthesize varying total directly via category sum.
            s.add(Category::Idle, Duration::from_micros((i * 100) as u64));
            s.end_frame();
        }
        // p95 of frame totals — totals come from actual elapsed time,
        // not the synth bucket, so this only asserts the API returns
        // *something*. Sanity check: not less than average.
        let avg = s.recent_avg_total(100).unwrap();
        let p95 = s.recent_p95_total(100).unwrap();
        assert!(p95 >= avg, "p95 {p95:?} should be ≥ avg {avg:?}");
    }
}
