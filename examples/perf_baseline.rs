//! Manual performance baseline for `Scene::tick` + `visible_entities`.
//!
//! `cargo run --example perf_baseline --release` measures the hot
//! loop at four roster sizes (10 / 25 / 50 / 100 entities), all
//! using the procedural fallback frame so there's no disk I/O. The
//! intent isn't to gate CI — it's to give a stable number we can
//! eyeball when investigating a regression report.
//!
//! Targets from `docs/engine-features.md` §7.1:
//! - Steady-state frame budget at 60 fps: 16.6 ms
//! - 100 entities under 8 ms per frame goal
//!
//! What this measures (sequentially):
//! - `Scene::tick(width, height, cursor)` — behavior + physics +
//!   animation advance for every entity
//! - `Scene::visible_entities()` — the lazily-cached visible/z-order
//!   filter (first call rebuilds the cache; subsequent calls are
//!   indices clone)
//!
//! What this *doesn't* measure:
//! - GPU upload / draw (needs a window)
//! - egui paint cost (needs a context)
//! - real asset decode (we use procedural placeholders)
//!
//! Adjust `iterations` if you want tighter or looser noise floors.

use anima_engine::animation::loader::generate_fallback_frame;
use anima_engine::animation::Animation;
use anima_engine::behavior::Behavior;
use anima_engine::config::{AssetType, CharacterConfig};
use anima_engine::entity::Entity;
use anima_engine::scene::Scene;
use std::time::Instant;

const SIZES: &[usize] = &[10, 25, 50, 100];
const ITERATIONS: usize = 1_000;

fn main() {
    println!(
        "anima_engine perf baseline\n\
         tick + visible_entities, {} iterations per size\n\
         budget @60fps: 16.6 ms / frame ; engine target: <8 ms @100 ent\n",
        ITERATIONS,
    );
    println!(
        "{:>8}  {:>10}  {:>10}  {:>10}",
        "entities", "tick avg", "visible avg", "tick+vis"
    );
    println!("{}", "-".repeat(46));

    for &n in SIZES {
        let mut scene = build_scene(n);
        // Warm-up: hot caches, jit allocations, etc.
        for _ in 0..50 {
            scene.tick(1920.0, 1080.0, Some((960.0, 540.0)));
            let _ = scene.visible_entities();
        }

        let mut tick_total = 0u128;
        let mut vis_total = 0u128;
        for _ in 0..ITERATIONS {
            let t0 = Instant::now();
            scene.tick(1920.0, 1080.0, Some((960.0, 540.0)));
            tick_total += t0.elapsed().as_micros();

            // Invalidate the cache every other call so we hit both the
            // rebuild and the fast path with a 50/50 mix.
            scene.mark_visible_dirty();
            let t1 = Instant::now();
            let _ = scene.visible_entities();
            vis_total += t1.elapsed().as_micros();
        }
        let tick_avg = tick_total as f64 / ITERATIONS as f64 / 1000.0;
        let vis_avg = vis_total as f64 / ITERATIONS as f64 / 1000.0;
        let combined = tick_avg + vis_avg;
        println!(
            "{:>8}  {:>8.3} ms  {:>8.3} ms  {:>8.3} ms",
            n, tick_avg, vis_avg, combined,
        );
    }
}

/// Build a scene of `n` entities pre-populated with fallback frames so
/// the loop never touches disk. Behaviors rotate through the 4
/// non-Reactive variants to exercise as many branches as possible.
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
        // 3 fallback frames so animation tick has something to advance.
        let frames: Vec<_> = (0..3)
            .map(|j| generate_fallback_frame([200, 200, 255, 200], 64 + j * 4))
            .collect();
        let animation = Animation::new(frames, cfg.fps, true);
        entities.push(Entity::from_config(&cfg, animation));
    }
    use anima_engine::config::AppConfig;
    let mut scene = Scene::from_config(&AppConfig::default());
    scene.entities = entities;
    scene
}
