//! Floating perf-overlay window (D.6).
//!
//! Reads the [`PerfSampler`](crate::perf::PerfSampler) and paints
//! the live frame-time + per-category breakdown. Top-right anchored,
//! always on top of the rest of the egui UI. Hidden by default;
//! toggled via `Action::TogglePerfOverlay` (`Ctrl+Shift+\`` default).
//!
//! Numbers shown:
//!
//! - **FPS** — derived from the 60-frame rolling average of total
//!   frame time.
//! - **Frame avg / p95** — average and 95th-percentile total frame
//!   time over the last 60 frames, in milliseconds.
//! - **Per category** — 60-frame rolling average for each of the
//!   five [`Category`](crate::perf::Category) buckets.
//! - **RSS** — resident-set size in MB, when available (Linux only,
//!   read from `/proc/self/status`).
//!
//! Snapshot export and RAM tracking live in [`crate::perf`] helpers;
//! the overlay only triggers them. A snapshot writes one chrome-
//! tracing JSON file at `~/.cache/animaEngine/perf-<unix_ts>.json`.

use crate::perf::{Category, PerfSampler};
use std::time::Duration;

/// Snapshot-export request emitted by the overlay. The caller (App)
/// handles the actual IO so the overlay function stays pure-UI.
pub struct ExportRequest;

/// Render the perf overlay. Returns `Some(ExportRequest)` when the
/// user clicks the export button this frame.
pub fn show(
    ctx: &egui::Context,
    sampler: &PerfSampler,
    rss_kib: Option<u64>,
) -> Option<ExportRequest> {
    let mut export_clicked = false;
    egui::Window::new("Perf")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .resizable(false)
        .collapsible(true)
        .default_open(true)
        .show(ctx, |ui| {
            // ── Frame-time block ─────────────────────────────────────
            let avg = sampler.recent_avg_total(60);
            let p95 = sampler.recent_p95_total(60);
            let fps = avg.and_then(|a| {
                let secs = a.as_secs_f64();
                (secs > 0.0).then_some(1.0 / secs)
            });
            ui.label(
                egui::RichText::new(match fps {
                    Some(f) => format!("FPS  {f:>6.1}"),
                    None => "FPS    —".into(),
                })
                .text_style(egui::TextStyle::Monospace)
                .strong(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "avg  {:>6.2} ms",
                    avg.map(ms).unwrap_or(0.0),
                ))
                .text_style(egui::TextStyle::Monospace),
            );
            ui.label(
                egui::RichText::new(format!(
                    "p95  {:>6.2} ms",
                    p95.map(ms).unwrap_or(0.0),
                ))
                .text_style(egui::TextStyle::Monospace),
            );
            ui.separator();

            // ── Per-category block ───────────────────────────────────
            for &cat in Category::ALL {
                let v = sampler.recent_avg_category(cat, 60);
                ui.label(
                    egui::RichText::new(format!(
                        "{:<13} {:>6.2} ms",
                        cat.label(),
                        v.map(ms).unwrap_or(0.0),
                    ))
                    .text_style(egui::TextStyle::Monospace),
                );
            }
            ui.separator();

            // ── RAM ──────────────────────────────────────────────────
            ui.label(
                egui::RichText::new(match rss_kib {
                    Some(kib) => {
                        let mib = kib as f64 / 1024.0;
                        format!("RSS  {mib:>6.1} MiB")
                    }
                    None => "RSS    —".into(),
                })
                .text_style(egui::TextStyle::Monospace),
            );
            ui.separator();

            // ── Sparkline-ish histogram (last 120 frame totals) ──────
            sparkline(ui, sampler);
            ui.separator();

            // ── Actions ──────────────────────────────────────────────
            ui.horizontal(|ui| {
                if ui.button("Export snapshot").clicked() {
                    export_clicked = true;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{} frames",
                        sampler.history_len()
                    ))
                    .weak()
                    .small(),
                );
            });
        });
    export_clicked.then_some(ExportRequest)
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Mini bar histogram of the last 120 frame totals. Tall bars
/// indicate stutters. Width and height are deliberately small so the
/// overlay stays compact.
fn sparkline(ui: &mut egui::Ui, sampler: &PerfSampler) {
    const N: usize = 120;
    const W: f32 = 180.0;
    const H: f32 = 28.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(W, H), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        2.0,
        ui.visuals().extreme_bg_color,
    );
    let samples: Vec<Duration> = sampler
        .history()
        .rev()
        .take(N)
        .map(|s| s.total)
        .collect();
    if samples.is_empty() {
        return;
    }
    // Scale: peg the bar height to the worst frame in the window;
    // skylines move dynamically. A horizontal line at 16.7 ms (60 fps
    // target) gives the eye a stable anchor.
    let max = samples
        .iter()
        .copied()
        .max()
        .unwrap_or(Duration::from_millis(16));
    let max_ms = ms(max).max(16.7);
    let bar_w = W / samples.len() as f32;
    let bar_color = ui.visuals().text_color();
    for (i, s) in samples.iter().rev().enumerate() {
        let h = (ms(*s) / max_ms) as f32 * H;
        let x = rect.left() + i as f32 * bar_w;
        let bar = egui::Rect::from_min_max(
            egui::pos2(x, rect.bottom() - h),
            egui::pos2(x + bar_w.max(1.0), rect.bottom()),
        );
        painter.rect_filled(bar, 0.0, bar_color);
    }
    // 16.7 ms reference line.
    let target_y = rect.bottom() - (16.7 / max_ms) as f32 * H;
    painter.line_segment(
        [
            egui::pos2(rect.left(), target_y),
            egui::pos2(rect.right(), target_y),
        ],
        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 180, 60)),
    );
}
