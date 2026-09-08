//! Aggregate machine load, for behavior scripts that react to it.
//!
//! # Why this reads two files instead of using a crate
//!
//! `sysinfo` is the obvious dependency, and it was rejected deliberately.
//! Its `system` feature bundles the process API with CPU and memory, so
//! there is no build-level way to prove we never enumerate processes —
//! only a promise not to call it. Process names and command lines say
//! exactly what a person is running; aggregate load says only that the
//! machine is busy. That difference is the whole privacy argument, and it
//! deserves to be enforced rather than asserted.
//!
//! Reading `/proc/stat` and `/proc/meminfo` directly makes it structural:
//! both contain nothing but totals, so this module *cannot* learn what is
//! running even if someone later wants it to. It also costs no dependency,
//! no binary size and no supply-chain surface.
//!
//! Nothing here leaves the process. It is exposed to scripts as two
//! numbers and never logged, persisted or transmitted — the zero-network
//! invariant in `docs/threat-model.md` covers the last of those.
//!
//! # Platforms
//!
//! Linux only for now. The BSDs do not expose a Linux-compatible
//! `/proc/stat`, and reporting a wrong number would be worse than
//! reporting none — elsewhere the readings stay at zero, which scripts see
//! as an idle machine.

use std::time::{Duration, Instant};

/// Minimum gap between samples.
///
/// CPU usage is a *rate*, so it only exists as a difference between two
/// readings; sampling every frame would divide by a near-zero window and
/// produce noise. Half a second is far below human perception for "the
/// machine got busy" and keeps this off the hot path.
const MIN_SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

/// Cumulative CPU jiffies from `/proc/stat`'s aggregate line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CpuTimes {
    idle: u64,
    total: u64,
}

/// Aggregate CPU and memory load, sampled on a throttle.
#[derive(Debug, Default)]
pub struct SystemLoad {
    last_sampled: Option<Instant>,
    previous: Option<CpuTimes>,
    cpu: f32,
    mem: f32,
}

impl SystemLoad {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fraction of CPU in use, 0.0–1.0. Zero until two samples exist, and
    /// on platforms without `/proc`.
    pub fn cpu(&self) -> f32 {
        self.cpu
    }

    /// Fraction of RAM in use, 0.0–1.0.
    pub fn mem(&self) -> f32 {
        self.mem
    }

    /// Re-read the counters if enough time has passed. Cheap to call every
    /// frame; the throttle is inside.
    pub fn refresh(&mut self) {
        let now = Instant::now();
        if let Some(last) = self.last_sampled {
            if now.duration_since(last) < MIN_SAMPLE_INTERVAL {
                return;
            }
        }
        self.last_sampled = Some(now);
        self.refresh_now();
    }

    #[cfg(target_os = "linux")]
    fn refresh_now(&mut self) {
        if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
            if let Some(current) = parse_cpu_times(&stat) {
                if let Some(prev) = self.previous {
                    if let Some(usage) = cpu_usage(prev, current) {
                        self.cpu = usage;
                    }
                }
                self.previous = Some(current);
            }
        }
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            if let Some(usage) = mem_usage(&meminfo) {
                self.mem = usage;
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn refresh_now(&mut self) {
        // No Linux-compatible /proc here. Leaving both at zero reads as an
        // idle machine, which is the honest answer to "we can't tell".
    }
}

/// Parse the aggregate `cpu` line of `/proc/stat`.
///
/// Fields are cumulative jiffies: user, nice, system, idle, iowait, irq,
/// softirq, steal, … `iowait` counts as idle — a machine waiting on disk
/// is not one a mascot should react to as busy.
fn parse_cpu_times(stat: &str) -> Option<CpuTimes> {
    let line = stat.lines().find(|l| l.starts_with("cpu "))?;
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|v| v.parse().ok())
        .collect();
    // user, nice, system, idle — the minimum that makes the ratio mean
    // anything. Kernels add fields over time, so take what is there.
    if values.len() < 4 {
        return None;
    }
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    Some(CpuTimes {
        idle,
        total: values.iter().sum(),
    })
}

/// Busy fraction between two `/proc/stat` readings.
///
/// Returns `None` when the window is empty or the counters moved
/// backwards, which happens across a suspend/resume.
fn cpu_usage(prev: CpuTimes, current: CpuTimes) -> Option<f32> {
    let total = current.total.checked_sub(prev.total)?;
    let idle = current.idle.checked_sub(prev.idle)?;
    if total == 0 || idle > total {
        return None;
    }
    Some(((total - idle) as f32 / total as f32).clamp(0.0, 1.0))
}

/// Used fraction from `/proc/meminfo`.
///
/// `MemAvailable` rather than `MemFree`: free memory excludes reclaimable
/// cache, so a healthy machine looks 95% full and every script would think
/// it is under pressure.
fn mem_usage(meminfo: &str) -> Option<f32> {
    let field = |name: &str| -> Option<u64> {
        meminfo
            .lines()
            .find(|l| l.starts_with(name))?
            .split_whitespace()
            .nth(1)?
            .parse()
            .ok()
    };
    let total = field("MemTotal:")?;
    let available = field("MemAvailable:")?;
    if total == 0 || available > total {
        return None;
    }
    Some(((total - available) as f32 / total as f32).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STAT: &str = "cpu  100 0 100 700 100 0 0 0 0 0\ncpu0 1 2 3 4\nintr 12345\n";

    #[test]
    fn parses_the_aggregate_cpu_line() {
        let t = parse_cpu_times(STAT).unwrap();
        // idle 700 + iowait 100; total is every field.
        assert_eq!(t.idle, 800);
        assert_eq!(t.total, 1000);
    }

    #[test]
    fn ignores_per_core_lines() {
        // `cpu0` must not be mistaken for the aggregate.
        let t = parse_cpu_times("cpu0 1 2 3 4\ncpu  10 0 10 80 0\n").unwrap();
        assert_eq!(t.total, 100);
    }

    #[test]
    fn rejects_a_truncated_cpu_line() {
        assert!(parse_cpu_times("cpu  1 2\n").is_none());
        assert!(parse_cpu_times("intr 5\n").is_none());
    }

    #[test]
    fn usage_is_the_non_idle_share_of_the_window() {
        let prev = CpuTimes {
            idle: 800,
            total: 1000,
        };
        let now = CpuTimes {
            idle: 850,
            total: 1100,
        };
        // 100 jiffies passed, 50 of them busy.
        assert!((cpu_usage(prev, now).unwrap() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn an_empty_window_has_no_answer() {
        let t = CpuTimes { idle: 1, total: 2 };
        assert_eq!(cpu_usage(t, t), None);
    }

    /// Counters go backwards across suspend/resume; that must not produce
    /// a nonsense reading or panic on the subtraction.
    #[test]
    fn counters_going_backwards_are_ignored() {
        let prev = CpuTimes {
            idle: 900,
            total: 1000,
        };
        let now = CpuTimes {
            idle: 10,
            total: 20,
        };
        assert_eq!(cpu_usage(prev, now), None);
    }

    #[test]
    fn idle_exceeding_the_window_is_rejected() {
        let prev = CpuTimes { idle: 0, total: 0 };
        let now = CpuTimes {
            idle: 200,
            total: 100,
        };
        assert_eq!(cpu_usage(prev, now), None);
    }

    #[test]
    fn memory_uses_available_not_free() {
        let info = "MemTotal:       1000 kB\nMemFree:          50 kB\nMemAvailable:    250 kB\n";
        // 750 of 1000 used — not the 950 that MemFree alone would imply.
        assert!((mem_usage(info).unwrap() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn missing_memory_fields_have_no_answer() {
        assert_eq!(mem_usage("MemTotal: 1000 kB\n"), None);
        assert_eq!(mem_usage(""), None);
    }

    #[test]
    fn readings_start_at_zero_and_stay_in_range() {
        let mut load = SystemLoad::new();
        assert_eq!(load.cpu(), 0.0);
        assert_eq!(load.mem(), 0.0);
        // Safe to call on any platform; on Linux it reads real files.
        load.refresh();
        assert!((0.0..=1.0).contains(&load.cpu()));
        assert!((0.0..=1.0).contains(&load.mem()));
    }

    /// The throttle is what keeps this off the hot path — a second call
    /// straight away must not re-read anything.
    #[test]
    fn refresh_is_throttled() {
        let mut load = SystemLoad::new();
        load.refresh();
        let first = load.last_sampled;
        load.refresh();
        assert_eq!(load.last_sampled, first, "sampled twice inside the window");
    }
}
