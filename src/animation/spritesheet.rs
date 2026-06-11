use super::frame::Frame;
use crate::constants::MAX_ANIMATION_FRAMES;
use crate::error::{AnimaError, Result};
use std::path::Path;

/// Load a spritesheet image and split it into a grid of animation frames.
///
/// A spritesheet is a single image containing multiple animation frames
/// arranged in a grid. Frames are read left-to-right, top-to-bottom.
///
/// # Arguments
/// * `path` — Path to the spritesheet image (PNG, WebP, etc.)
/// * `columns` — Number of columns in the grid
/// * `rows` — Number of rows in the grid
///
/// # Example
/// A 512x256 spritesheet with 4 columns and 2 rows produces 8 frames of 128x128 each.
pub fn load_spritesheet(path: &Path, columns: u32, rows: u32) -> Result<Vec<Frame>> {
    if columns == 0 || rows == 0 {
        return Err(AnimaError::InvalidSpritesheet(
            "columns and rows must be > 0".into(),
        ));
    }

    let img = image::open(path)?.to_rgba8();
    let (img_width, img_height) = img.dimensions();

    let cell_width = img_width / columns;
    let cell_height = img_height / rows;

    if cell_width == 0 || cell_height == 0 {
        return Err(AnimaError::InvalidSpritesheet(format!(
            "cells too small: image is {}x{}, grid is {}x{} → cell would be {}x{}",
            img_width, img_height, columns, rows, cell_width, cell_height
        )));
    }

    // Slice the atlas directly out of its raw pixel buffer. Each row of a
    // cell is a contiguous memcpy from the source image — orders of magnitude
    // faster than per-pixel `get_pixel` for large sheets (a 2048×2048 / 8×8
    // sheet drops from ~4 million function calls to ~16k memcpys).
    let raw = img.as_raw();
    let row_stride = (img_width as usize) * 4;
    let cell_row_bytes = (cell_width as usize) * 4;
    let cell_byte_size = cell_row_bytes * cell_height as usize;

    // Cap the slice count *during* the loop, not post-hoc — the same
    // G.4 rationale as the PNG-sequence loader. The zero-cell check
    // above bounds the grid at img_width × img_height cells (≤ 4096²
    // with MAX_IMAGE_DIM), and 16.7 M one-pixel Frame structs is ~1 GB
    // of transient allocation a hand-edited config could otherwise
    // request before any post-hoc truncation ran.
    let grid_cells = (columns as usize) * (rows as usize);
    let keep = grid_cells.min(MAX_ANIMATION_FRAMES);
    let mut frames = Vec::with_capacity(keep);

    'slice: for row in 0..rows {
        for col in 0..columns {
            if frames.len() >= MAX_ANIMATION_FRAMES {
                tracing::warn!(
                    "Spritesheet {} truncated at MAX_ANIMATION_FRAMES = {} ({}x{} grid requested)",
                    crate::drop_validate::redact_path(path),
                    MAX_ANIMATION_FRAMES,
                    columns,
                    rows
                );
                break 'slice;
            }
            let x_offset = (col * cell_width) as usize;
            let y_offset = (row * cell_height) as usize;

            let mut rgba = Vec::with_capacity(cell_byte_size);
            for y in 0..cell_height as usize {
                let src_y = y_offset + y;
                let src_start = src_y * row_stride + x_offset * 4;
                rgba.extend_from_slice(&raw[src_start..src_start + cell_row_bytes]);
            }

            frames.push(Frame::new(rgba, cell_width, cell_height));
        }
    }

    tracing::info!(
        "Loaded spritesheet: {}x{} grid ({} frames of {}x{}) from {}",
        columns,
        rows,
        frames.len(),
        cell_width,
        cell_height,
        crate::drop_validate::redact_path(path)
    );
    tracing::debug!("Spritesheet full path: {}", path.display());

    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("spritesheet_tests")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Build a 4×2 spritesheet (4 cols, 2 rows) of 8×8 cells where each cell
    /// is filled with a unique solid red value matching its index. Cells are
    /// numbered left-to-right, top-to-bottom: (0,0)=10, (1,0)=20, …, (3,1)=80.
    fn write_indexed_sheet(path: &Path) {
        let cell = 8u32;
        let cols = 4u32;
        let rows = 2u32;
        let mut img = image::RgbaImage::new(cols * cell, rows * cell);
        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let color = image::Rgba([((idx + 1) as u8) * 10, 0, 0, 255]);
                for y in 0..cell {
                    for x in 0..cell {
                        img.put_pixel(col * cell + x, row * cell + y, color);
                    }
                }
            }
        }
        img.save(path).unwrap();
    }

    #[test]
    fn grid_larger_than_frame_cap_truncates_during_slice() {
        // 64×16 grid of 1×1 cells = 1024 requested frames, well past
        // MAX_ANIMATION_FRAMES (600). The cap must apply during the
        // slice loop, bounding transient allocation, not after.
        let dir = temp_dir("cap_sheet");
        let path = dir.join("big.png");
        let img = image::RgbaImage::new(64, 16);
        img.save(&path).unwrap();

        let frames = load_spritesheet(&path, 64, 16).unwrap();
        assert_eq!(frames.len(), MAX_ANIMATION_FRAMES);
    }

    #[test]
    fn slicing_preserves_cell_boundaries_and_order() {
        let dir = temp_dir("indexed_sheet");
        let path = dir.join("sheet.png");
        write_indexed_sheet(&path);

        let frames = load_spritesheet(&path, 4, 2).unwrap();
        assert_eq!(frames.len(), 8);

        // Each cell must be uniformly colored with its index marker.
        // Any off-by-one in the row_stride math would smear two cells together.
        for (idx, frame) in frames.iter().enumerate() {
            let expected_r = ((idx + 1) as u8) * 10;
            assert_eq!(frame.width, 8);
            assert_eq!(frame.height, 8);
            for chunk in frame.rgba.chunks_exact(4) {
                assert_eq!(
                    chunk[0], expected_r,
                    "cell {idx}: pixel red channel mismatch"
                );
                assert_eq!(chunk[3], 255, "cell {idx}: alpha mismatch");
            }
        }
    }
}
