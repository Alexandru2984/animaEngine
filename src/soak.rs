//! Soak-test metrics emitter (W.1, 0.9).
//!
//! Off by default and zero-cost unless `ANIMA_SOAK_METRICS=<path>` is
//! set. When it is, the render loop appends one CSV row per interval
//! (`ANIMA_SOAK_INTERVAL_SECS`, default 60) carrying the numbers
//! `scripts/soak.sh` regresses for leak detection: resident-set size,
//! total decoded asset bytes, live GPU texture count, and the 60-frame
//! p95 frame time.
//!
//! The emitter is ticked once per frame; it writes only when the
//! interval has elapsed, so the synthetic soak scene must keep
//! animating (behaviours on) for sampling to stay regular. Each row is
//! flushed immediately so an externally-killed run still has every
//! sample on disk.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// One periodic sampler writing CSV rows to a file. Construct with
/// [`SoakRecorder::from_env`]; `None` keeps the whole feature inert.
pub struct SoakRecorder {
    writer: BufWriter<File>,
    interval: Duration,
    start: Instant,
    last_emit: Instant,
    samples: u64,
}

impl SoakRecorder {
    /// `Some` only when `ANIMA_SOAK_METRICS` names a writable path.
    /// Truncates the file and writes the CSV header. A path that can't
    /// be opened logs a warning and returns `None` (the app runs
    /// normally, just without soak output).
    pub fn from_env() -> Option<Self> {
        let path: PathBuf = std::env::var_os("ANIMA_SOAK_METRICS")?.into();
        let secs = std::env::var("ANIMA_SOAK_INTERVAL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(60);

        let file = match File::create(&path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(
                    "ANIMA_SOAK_METRICS set but {} is not writable: {e}",
                    path.display()
                );
                return None;
            }
        };
        let mut writer = BufWriter::new(file);
        // Header doubles as the column contract for scripts/soak.sh.
        if let Err(e) = writeln!(
            writer,
            "elapsed_secs,rss_kib,decoded_bytes,texture_count,frame_p95_us"
        ) {
            tracing::warn!("soak: header write failed: {e}");
            return None;
        }
        let _ = writer.flush();
        tracing::info!(
            "Soak metrics enabled → {} (every {secs}s)",
            crate::drop_validate::redact_path(&path)
        );

        let now = Instant::now();
        Some(Self {
            writer,
            interval: Duration::from_secs(secs),
            start: now,
            last_emit: now,
            samples: 0,
        })
    }

    /// Call once per rendered frame. Emits a row only when the interval
    /// has elapsed since the last one. Cheap on the common (no-emit)
    /// path: one `Instant::now()` and a compare.
    pub fn maybe_sample(
        &mut self,
        rss_kib: Option<u64>,
        decoded_bytes: usize,
        texture_count: usize,
        frame_p95_us: Option<u128>,
    ) {
        let now = Instant::now();
        if now.duration_since(self.last_emit) < self.interval {
            return;
        }
        self.last_emit = now;
        self.samples += 1;
        let elapsed = now.duration_since(self.start).as_secs();
        let row = format!(
            "{elapsed},{},{decoded_bytes},{texture_count},{}",
            rss_kib.map(|v| v.to_string()).unwrap_or_default(),
            frame_p95_us.map(|v| v.to_string()).unwrap_or_default(),
        );
        if let Err(e) = writeln!(self.writer, "{row}") {
            tracing::warn!("soak: row write failed: {e}");
            return;
        }
        let _ = self.writer.flush();
        tracing::debug!("soak sample {} @ {elapsed}s: {row}", self.samples);
    }

    /// Test seam: build a recorder writing to `path` with the given
    /// interval, bypassing the env lookup.
    #[cfg(test)]
    fn for_test(path: &std::path::Path, interval: Duration) -> Self {
        let mut writer = BufWriter::new(File::create(path).unwrap());
        writeln!(
            writer,
            "elapsed_secs,rss_kib,decoded_bytes,texture_count,frame_p95_us"
        )
        .unwrap();
        writer.flush().unwrap();
        let now = Instant::now();
        Self {
            writer,
            interval,
            start: now,
            last_emit: now,
            samples: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("anima_soak_{}_{}.csv", name, std::process::id()))
    }

    #[test]
    fn header_then_rows_with_zero_interval() {
        let path = tmp("zero");
        let _ = std::fs::remove_file(&path);
        // Zero interval → every call emits.
        let mut rec = SoakRecorder::for_test(&path, Duration::ZERO);
        rec.maybe_sample(Some(1000), 2048, 4, Some(16_600));
        rec.maybe_sample(Some(1001), 2048, 4, None);
        drop(rec);

        let body = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(
            lines[0],
            "elapsed_secs,rss_kib,decoded_bytes,texture_count,frame_p95_us"
        );
        assert_eq!(lines.len(), 3, "header + 2 rows: {body}");
        // First row carries all fields; second has an empty p95 cell.
        assert!(lines[1].ends_with(",1000,2048,4,16600"));
        assert!(
            lines[2].ends_with(",1001,2048,4,"),
            "missing p95 → empty cell: {}",
            lines[2]
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn interval_suppresses_until_elapsed() {
        let path = tmp("suppress");
        let _ = std::fs::remove_file(&path);
        // Large interval → no row emitted within the test's lifetime.
        let mut rec = SoakRecorder::for_test(&path, Duration::from_secs(3600));
        rec.maybe_sample(Some(1000), 0, 0, None);
        rec.maybe_sample(Some(1000), 0, 0, None);
        drop(rec);

        let lines = std::fs::read_to_string(&path).unwrap().lines().count();
        assert_eq!(lines, 1, "only the header — interval not elapsed");
        let _ = std::fs::remove_file(&path);
    }
}
