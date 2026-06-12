//! Criterion benchmarks for the per-frame hot paths and the startup
//! cache codec. Everything is synthetic (procedural frames, no disk,
//! no GPU) so the numbers isolate CPU work and stay comparable across
//! machines.
//!
//! Run: `cargo bench` — or `cargo bench -- --quick` for a fast pass.
//! Compare against the targets in docs/engine-features.md §7.1
//! (100 entities < 8 ms/frame; the scene_tick/100 number here is the
//! CPU share of that budget).

use anima_engine::animation::cache::{deserialize_frames, serialize_frames};
use anima_engine::animation::loader::generate_fallback_frame;
use anima_engine::animation::Animation;
use anima_engine::behavior::Behavior;
use anima_engine::config::{AppConfig, AssetType, CharacterConfig};
use anima_engine::entity::Entity;
use anima_engine::group::GroupConfig;
use anima_engine::monitor::{plan_windows, MonitorInfo, MonitorMode};
use anima_engine::scene::Scene;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

/// Mirror of examples/perf_baseline.rs `build_scene` — synthetic
/// entities with rotating behaviors and 3 procedural frames each.
fn build_scene(n: usize) -> Scene {
    let mut entities = Vec::with_capacity(n);
    for i in 0..n {
        let behavior = match i % 4 {
            0 => Behavior::Idle,
            1 => Behavior::WalkAround { speed: 60.0 },
            2 => Behavior::BoundedWander {
                x_min: 0.0,
                x_max: 1920.0,
                y_min: 0.0,
                y_max: 1080.0,
                speed: 120.0,
            },
            _ => Behavior::Bounce {
                amplitude_px: 24.0,
                period_sec: 1.5,
                axis: anima_engine::behavior::BounceAxis::Vertical,
            },
        };
        let cfg = CharacterConfig {
            id: format!("e_{i}"),
            name: format!("Entity {i}"),
            asset_type: AssetType::PngStatic,
            asset_path: String::new(),
            x: (i as f32 * 7.0) % 1920.0,
            y: (i as f32 * 11.0) % 1080.0,
            scale: 1.0,
            opacity: 1.0,
            fps: 8.0,
            visible: true,
            playing: false,
            z_index: i as i32,
            physics_enabled: false,
            behavior,
            spritesheet_columns: None,
            spritesheet_rows: None,
            monitor: None,
            easing: None,
            animations: std::collections::BTreeMap::new(),
        };
        let frames: Vec<_> = (0..3)
            .map(|j| generate_fallback_frame([200, 200, 255, 200], 64 + j * 4))
            .collect();
        let animation = Animation::new(frames, cfg.fps, true);
        entities.push(Entity::from_config(&cfg, animation));
    }
    let mut scene = Scene::from_config(&AppConfig::default());
    scene.entities = entities;
    scene
}

fn bench_scene_tick(c: &mut Criterion) {
    let mut g = c.benchmark_group("scene_tick");
    for &n in &[10usize, 50, 100] {
        let mut scene = build_scene(n);
        g.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| scene.tick(1920.0, 1080.0, Some((960.0, 540.0))));
        });
    }
    g.finish();
}

fn bench_visible_entities(c: &mut Criterion) {
    let mut scene = build_scene(100);
    scene.tick(1920.0, 1080.0, None);
    c.bench_function("visible_entities_rebuild/100", |b| {
        b.iter(|| {
            scene.mark_visible_dirty();
            black_box(scene.visible_entities().len())
        });
    });
}

fn bench_cache_codec(c: &mut Criterion) {
    // 16 frames of 128×128 RGBA ≈ 1 MiB — a mid-size GIF's worth.
    let frames: Vec<_> = (0..16)
        .map(|i| generate_fallback_frame([i as u8 * 16, 128, 255, 255], 128))
        .collect();
    let bytes = serialize_frames(&frames);
    c.bench_function("cache_serialize/16x128", |b| {
        b.iter(|| black_box(serialize_frames(black_box(&frames))).len())
    });
    c.bench_function("cache_deserialize/16x128", |b| {
        b.iter(|| deserialize_frames(black_box(&bytes)).unwrap().len())
    });
}

fn monitors() -> Vec<MonitorInfo> {
    (0..3)
        .map(|i| MonitorInfo {
            name: format!("HDMI-A-{i}"),
            x: i * 1920,
            y: 0,
            width: 1920,
            height: 1080,
            scale_factor: 1.0,
            is_primary: i == 0,
        })
        .collect()
}

fn bench_plan_windows(c: &mut Criterion) {
    let mons = monitors();
    c.bench_function("plan_windows/per_monitor_3", |b| {
        b.iter(|| black_box(plan_windows(&MonitorMode::PerMonitor, black_box(&mons))))
    });
}

fn bench_group_transform(c: &mut Criterion) {
    // 8 groups × 8 members; the looked-up entity sits in the last
    // group so the scan pays the full price (worst case).
    let groups: Vec<GroupConfig> = (0..8)
        .map(|gi| GroupConfig {
            id: format!("g{gi}"),
            member_ids: (0..8).map(|m| format!("e_{gi}_{m}")).collect(),
            offset_x: gi as f32,
            scale: 1.25,
            ..Default::default()
        })
        .collect();
    c.bench_function("group_transform/worst_of_64", |b| {
        b.iter(|| {
            black_box(anima_engine::group::transform_for_member(
                black_box(&groups),
                black_box("e_7_7"),
            ))
        })
    });
}

criterion_group!(
    benches,
    bench_scene_tick,
    bench_visible_entities,
    bench_cache_codec,
    bench_plan_windows,
    bench_group_transform
);
criterion_main!(benches);
