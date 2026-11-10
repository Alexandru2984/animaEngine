//! Every frame of every built-in character, in every state, drawn `scale`
//! times the app's 128 px, for `scripts/site-assets.py`: a website's
//! characters — stills, animations, sprite sheets — as sharp as the app
//! draws them. Written as `<folder>/<name>/<state>/NN.png`, the states
//! `idle` and the folders a character brings (`walk`, `fall`, `drag`,
//! `sleep`).
//!
//!     cargo run --release --example site_pictures -- <folder> [scale]

use anima_engine::animation::StateId;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = std::path::PathBuf::from(args.next().expect("a folder to write into"));
    let scale: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(2);
    for name in anima_engine::demo::CHARACTERS {
        let states =
            std::iter::once((StateId::Idle, "idle")).chain(StateId::FOLDERS.iter().copied());
        for (state, folder) in states {
            let into = dir.join(name).join(folder);
            std::fs::create_dir_all(&into).expect("make the folder");
            for frame in 0..anima_engine::demo::frames_of(state) {
                anima_engine::demo::picture_in(name, state, frame, scale)
                    .expect("a built-in character")
                    .save(into.join(format!("{frame:02}.png")))
                    .expect("write the picture");
            }
        }
    }
}
