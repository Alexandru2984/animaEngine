//! The built-in characters drawn large, for `scripts/store-art.py`: a PNG
//! for each character at four points of its loop, `scale` times the app's
//! 128 px (4 by default: 512 px), as sharp as the app's own frames — and at
//! the same points walking, falling and held (`<name>-walk-<frame>.png`).
//!
//!     cargo run --release --example store_pictures -- <folder> [scale]

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("a folder to write into"));
    let scale: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    std::fs::create_dir_all(&dir).expect("make the folder");
    for name in anima_engine::demo::CHARACTERS {
        for frame in [0, 3, 6, 9] {
            let picture =
                anima_engine::demo::picture(name, frame, scale).expect("a built-in character");
            picture
                .save(dir.join(format!("{name}-{frame}.png")))
                .expect("write the picture");
            for (state, folder) in anima_engine::animation::StateId::FOLDERS {
                anima_engine::demo::picture_in(name, state, frame, scale)
                    .expect("a built-in character")
                    .save(dir.join(format!("{name}-{folder}-{frame}.png")))
                    .expect("write the picture");
            }
        }
    }
}
