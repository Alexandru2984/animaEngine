use std::fs;
use std::path::PathBuf;

/// Helper: create a temporary directory with a unique name inside the project
fn temp_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test_assets")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("Failed to create temp test dir");
    dir
}

/// Helper: generate a small test PNG image and save it
fn create_test_png(path: &std::path::Path, width: u32, height: u32, color: [u8; 4]) {
    let mut img = image::RgbaImage::new(width, height);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba(color);
    }
    img.save(path).expect("Failed to save test PNG");
}

/// Helper: generate a small test GIF file with multiple frames
fn create_test_gif(path: &std::path::Path, frame_count: u32) {
    use image::codecs::gif::GifEncoder;
    use image::{Frame, RgbaImage};
    use std::fs::File;

    let file = File::create(path).expect("Failed to create GIF file");
    let mut encoder = GifEncoder::new(file);

    for i in 0..frame_count {
        let brightness = 50 + (i * 50);
        let mut img = RgbaImage::new(16, 16);
        for pixel in img.pixels_mut() {
            *pixel = image::Rgba([brightness as u8, 100, 200, 255]);
        }
        let frame = Frame::new(img);
        encoder
            .encode_frames(std::iter::once(frame))
            .expect("Failed to encode GIF frame");
    }
}

// ============================================================================
// Frame tests
// ============================================================================

#[test]
fn test_frame_new_has_no_delay() {
    use anima_engine::animation::frame::Frame;

    let rgba = vec![0u8; 64 * 64 * 4];
    let frame = Frame::new(rgba, 64, 64);
    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 64);
    assert_eq!(frame.delay_ms, None);
}

#[test]
fn test_frame_with_delay() {
    use anima_engine::animation::frame::Frame;

    let rgba = vec![0u8; 32 * 32 * 4];
    let frame = Frame::with_delay(rgba, 32, 32, 100);
    assert_eq!(frame.width, 32);
    assert_eq!(frame.height, 32);
    assert_eq!(frame.delay_ms, Some(100));
}

// ============================================================================
// PNG sequence loading tests
// ============================================================================

#[test]
fn test_load_png_sequence() {
    use anima_engine::animation::png_sequence;

    let dir = temp_dir("png_sequence");

    // Create 3 test PNG frames
    for i in 1..=3 {
        let path = dir.join(format!("frame_{:03}.png", i));
        create_test_png(&path, 32, 32, [255, 0, 0, 255]);
    }

    let frames = png_sequence::load_png_sequence(&dir).expect("Should load PNG sequence");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].width, 32);
    assert_eq!(frames[0].height, 32);
    // PNG frames should not have per-frame delays
    assert!(frames.iter().all(|f| f.delay_ms.is_none()));
}

#[test]
fn test_load_single_png() {
    use anima_engine::animation::png_sequence;

    let dir = temp_dir("single_png");
    let path = dir.join("test.png");
    create_test_png(&path, 64, 48, [0, 255, 0, 200]);

    let frame = png_sequence::load_single_png(&path).expect("Should load single PNG");
    assert_eq!(frame.width, 64);
    assert_eq!(frame.height, 48);
}

#[test]
fn test_load_png_sequence_empty_dir() {
    use anima_engine::animation::png_sequence;

    let dir = temp_dir("png_empty");
    // No PNGs in directory
    let result = png_sequence::load_png_sequence(&dir);
    assert!(result.is_err());
}

#[test]
fn test_load_png_sequence_ignores_non_png() {
    use anima_engine::animation::png_sequence;

    let dir = temp_dir("png_mixed");

    // Create one PNG and one non-PNG file
    create_test_png(&dir.join("frame_001.png"), 16, 16, [255, 255, 255, 255]);
    fs::write(dir.join("readme.txt"), "not a PNG").expect("write txt");

    let frames = png_sequence::load_png_sequence(&dir).expect("Should load PNG sequence");
    assert_eq!(frames.len(), 1); // Only the PNG file
}

// ============================================================================
// GIF loading tests
// ============================================================================

#[test]
fn test_load_gif_basic() {
    use anima_engine::animation::gif_loader;

    let dir = temp_dir("gif_basic");
    let path = dir.join("test.gif");
    create_test_gif(&path, 3);

    let frames = gif_loader::load_gif(&path).expect("Should load GIF");
    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].width, 16);
    assert_eq!(frames[0].height, 16);
}

#[test]
fn test_load_gif_missing_file() {
    use anima_engine::animation::gif_loader;

    let result = gif_loader::load_gif(std::path::Path::new("/nonexistent/test.gif"));
    assert!(result.is_err());
}

// ============================================================================
// Spritesheet tests
// ============================================================================

#[test]
fn test_load_spritesheet_basic() {
    use anima_engine::animation::spritesheet;

    let dir = temp_dir("spritesheet_basic");
    let path = dir.join("sheet.png");

    // Create a 64x32 image (4 columns, 2 rows = 8 frames of 16x16)
    create_test_png(&path, 64, 32, [100, 150, 200, 255]);

    let frames = spritesheet::load_spritesheet(&path, 4, 2).expect("Should load spritesheet");
    assert_eq!(frames.len(), 8);
    assert_eq!(frames[0].width, 16);
    assert_eq!(frames[0].height, 16);
}

#[test]
fn test_load_spritesheet_single_row() {
    use anima_engine::animation::spritesheet;

    let dir = temp_dir("spritesheet_single_row");
    let path = dir.join("strip.png");

    // Create a 128x32 image (4 columns, 1 row = 4 frames of 32x32)
    create_test_png(&path, 128, 32, [50, 100, 150, 255]);

    let frames = spritesheet::load_spritesheet(&path, 4, 1).expect("Should load strip");
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].width, 32);
    assert_eq!(frames[0].height, 32);
}

#[test]
fn test_load_spritesheet_zero_columns() {
    use anima_engine::animation::spritesheet;

    let dir = temp_dir("spritesheet_zero");
    let path = dir.join("sheet.png");
    create_test_png(&path, 64, 64, [255, 255, 255, 255]);

    let result = spritesheet::load_spritesheet(&path, 0, 2);
    assert!(result.is_err());
}

#[test]
fn test_load_spritesheet_zero_rows() {
    use anima_engine::animation::spritesheet;

    let dir = temp_dir("spritesheet_zero_rows");
    let path = dir.join("sheet.png");
    create_test_png(&path, 64, 64, [255, 255, 255, 255]);

    let result = spritesheet::load_spritesheet(&path, 2, 0);
    assert!(result.is_err());
}

// ============================================================================
// Animation tests
// ============================================================================

#[test]
fn test_animation_detects_per_frame_delays() {
    use anima_engine::animation::frame::Frame;
    use anima_engine::animation::Animation;

    let frames = vec![
        Frame::with_delay(vec![0; 16], 2, 2, 100),
        Frame::with_delay(vec![0; 16], 2, 2, 200),
    ];
    let anim = Animation::new(frames, 12.0, true);
    assert!(anim.has_per_frame_delays);
}

#[test]
fn test_animation_no_per_frame_delays() {
    use anima_engine::animation::frame::Frame;
    use anima_engine::animation::Animation;

    let frames = vec![Frame::new(vec![0; 16], 2, 2), Frame::new(vec![0; 16], 2, 2)];
    let anim = Animation::new(frames, 12.0, true);
    assert!(!anim.has_per_frame_delays);
}

#[test]
fn test_animation_single_frame_no_tick() {
    use anima_engine::animation::frame::Frame;
    use anima_engine::animation::Animation;

    let frames = vec![Frame::new(vec![0; 16], 2, 2)];
    let mut anim = Animation::new(frames, 12.0, true);
    // Single frame should never advance
    assert!(!anim.tick());
}

#[test]
fn test_animation_paused_no_tick() {
    use anima_engine::animation::frame::Frame;
    use anima_engine::animation::Animation;

    let frames = vec![Frame::new(vec![0; 16], 2, 2), Frame::new(vec![0; 16], 2, 2)];
    let mut anim = Animation::new(frames, 12.0, false); // paused
    std::thread::sleep(std::time::Duration::from_millis(200));
    assert!(!anim.tick());
}

#[test]
fn test_animation_toggle_playback() {
    use anima_engine::animation::frame::Frame;
    use anima_engine::animation::Animation;

    let frames = vec![Frame::new(vec![0; 16], 2, 2)];
    let mut anim = Animation::new(frames, 12.0, false);
    assert!(!anim.playing);
    anim.toggle_playback();
    assert!(anim.playing);
    anim.toggle_playback();
    assert!(!anim.playing);
}

#[test]
fn test_animation_set_fps() {
    use anima_engine::animation::frame::Frame;
    use anima_engine::animation::Animation;

    let frames = vec![Frame::new(vec![0; 16], 2, 2)];
    let mut anim = Animation::new(frames, 12.0, true);
    anim.set_fps(30.0);
    assert!((anim.fps - 30.0).abs() < f32::EPSILON);
    // Test minimum clamp
    anim.set_fps(-5.0);
    assert!((anim.fps - 0.1).abs() < f32::EPSILON);
}

// ============================================================================
// Fallback frame tests
// ============================================================================

#[test]
fn test_generate_fallback_frame() {
    use anima_engine::animation::loader::generate_fallback_frame;

    let frame = generate_fallback_frame([255, 128, 64, 200], 32);
    assert_eq!(frame.width, 32);
    assert_eq!(frame.height, 32);
    assert_eq!(frame.rgba.len(), (32 * 32 * 4) as usize);
}

// ============================================================================
// Config serialization tests
// ============================================================================

#[test]
fn test_config_default_has_demo_characters() {
    use anima_engine::config::AppConfig;

    let config = AppConfig::default();
    let ids: Vec<&str> = config.characters.iter().map(|c| c.id.as_str()).collect();
    // Sample pack — five procedural demos. Order matters because z_index
    // is currently derived from listing order.
    assert_eq!(ids, vec!["ghost", "slime", "heart", "star", "cat"]);
}

#[test]
fn test_config_roundtrip_serialization() {
    use anima_engine::config::{AppConfig, AssetType};

    let config = AppConfig::default();
    let toml_str = toml::to_string_pretty(&config).expect("serialize");
    let loaded: AppConfig = toml::from_str(&toml_str).expect("deserialize");

    assert_eq!(loaded.characters.len(), config.characters.len());
    assert_eq!(loaded.characters[0].asset_type, AssetType::PngSequence);
    assert_eq!(
        loaded.global.playback_enabled,
        config.global.playback_enabled
    );
}

#[test]
fn test_config_new_asset_types_serialize() {
    use anima_engine::config::{AppConfig, AssetType, CharacterConfig, GlobalConfig};

    let config = AppConfig {
        global: GlobalConfig::default(),
        characters: vec![
            CharacterConfig {
                id: "test_webp".to_string(),
                name: "WebP Test".to_string(),
                asset_type: AssetType::WebpAnimated,
                asset_path: "assets/test.webp".to_string(),
                x: 0.0,
                y: 0.0,
                scale: 1.0,
                opacity: 1.0,
                fps: 12.0,
                visible: true,
                playing: true,
                z_index: 0,
                physics_enabled: false,
                behavior: anima_engine::behavior::Behavior::Idle,
                spritesheet_columns: None,
                spritesheet_rows: None,
            },
            CharacterConfig {
                id: "test_sheet".to_string(),
                name: "Sheet Test".to_string(),
                asset_type: AssetType::Spritesheet,
                asset_path: "assets/sheet.png".to_string(),
                x: 100.0,
                y: 100.0,
                scale: 2.0,
                opacity: 0.8,
                fps: 24.0,
                visible: true,
                playing: true,
                z_index: 5,
                physics_enabled: false,
                behavior: anima_engine::behavior::Behavior::Idle,
                spritesheet_columns: Some(4),
                spritesheet_rows: Some(2),
            },
        ],
    };

    // Serialize as full AppConfig (TOML requires a table at the root, not a bare array)
    let toml_str = toml::to_string_pretty(&config).expect("serialize");
    // Deserialize
    let loaded: AppConfig = toml::from_str(&toml_str).expect("deserialize");

    assert_eq!(loaded.characters[0].asset_type, AssetType::WebpAnimated);
    assert_eq!(loaded.characters[1].asset_type, AssetType::Spritesheet);
    assert_eq!(loaded.characters[1].spritesheet_columns, Some(4));
    assert_eq!(loaded.characters[1].spritesheet_rows, Some(2));

    // WebP entry should NOT have spritesheet fields (they're None → skipped)
    // Spritesheet entry SHOULD have them
    assert!(toml_str.contains("spritesheet_columns = 4"));
    assert!(toml_str.contains("spritesheet_rows = 2"));
}

#[test]
fn test_legacy_config_without_physics_field_defaults_to_disabled() {
    // Configs written before physics_enabled was introduced must still parse,
    // with physics_enabled defaulting to false (i.e. legacy behavior).
    use anima_engine::config::{AppConfig, AssetType};

    let legacy_toml = r#"
[global]
always_on_top = true
transparent = true
playback_enabled = true
window_width = 0
window_height = 0

[[characters]]
id = "legacy"
name = "Legacy"
asset_type = "png_static"
asset_path = "x.png"
x = 0.0
y = 0.0
"#;

    let cfg: AppConfig = toml::from_str(legacy_toml).expect("legacy parse");
    assert_eq!(cfg.characters[0].asset_type, AssetType::PngStatic);
    assert!(
        !cfg.characters[0].physics_enabled,
        "missing physics_enabled must default to false"
    );
}

#[test]
fn test_config_spritesheet_fields_skip_when_none() {
    use anima_engine::config::{AssetType, CharacterConfig};

    let config = CharacterConfig {
        id: "no_sheet".to_string(),
        name: "No Sheet".to_string(),
        asset_type: AssetType::PngStatic,
        asset_path: "test.png".to_string(),
        x: 0.0,
        y: 0.0,
        scale: 1.0,
        opacity: 1.0,
        fps: 12.0,
        visible: true,
        playing: true,
        z_index: 0,
        physics_enabled: false,
        behavior: anima_engine::behavior::Behavior::Idle,
        spritesheet_columns: None,
        spritesheet_rows: None,
    };

    let toml_str = toml::to_string_pretty(&config).expect("serialize");
    // spritesheet fields should NOT appear when None
    assert!(!toml_str.contains("spritesheet_columns"));
    assert!(!toml_str.contains("spritesheet_rows"));
}

#[test]
fn test_detect_asset_type_gif() {
    use anima_engine::config::{AppConfig, AssetType};
    assert_eq!(
        AppConfig::detect_asset_type("animation.gif"),
        AssetType::Gif
    );
}

#[test]
fn test_detect_asset_type_webp() {
    use anima_engine::config::{AppConfig, AssetType};
    assert_eq!(
        AppConfig::detect_asset_type("sprite.webp"),
        AssetType::WebpAnimated
    );
}

#[test]
fn test_detect_asset_type_png() {
    use anima_engine::config::{AppConfig, AssetType};
    assert_eq!(
        AppConfig::detect_asset_type("image.png"),
        AssetType::PngStatic
    );
}

#[test]
fn test_detect_asset_type_unknown() {
    use anima_engine::config::{AppConfig, AssetType};
    assert_eq!(
        AppConfig::detect_asset_type("something.bmp"),
        AssetType::PngStatic
    );
}

// ============================================================================
// Asset type detection from loader module
// ============================================================================

#[test]
fn test_loader_detect_asset_type() {
    use anima_engine::animation::loader::detect_asset_type;
    use anima_engine::config::AssetType;
    use std::path::Path;

    let (t, _) = detect_asset_type(Path::new("character.gif"));
    assert_eq!(t, AssetType::Gif);

    let (t, _) = detect_asset_type(Path::new("sprite.webp"));
    assert_eq!(t, AssetType::WebpAnimated);

    let (t, _) = detect_asset_type(Path::new("icon.png"));
    assert_eq!(t, AssetType::PngStatic);
}
