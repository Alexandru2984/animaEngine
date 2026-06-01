use super::frame::Frame;
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

    let mut frames = Vec::with_capacity((columns * rows) as usize);

    for row in 0..rows {
        for col in 0..columns {
            let x_offset = col * cell_width;
            let y_offset = row * cell_height;

            // Extract the cell pixels
            let mut rgba = Vec::with_capacity((cell_width * cell_height * 4) as usize);
            for y in 0..cell_height {
                for x in 0..cell_width {
                    let pixel = img.get_pixel(x_offset + x, y_offset + y);
                    rgba.extend_from_slice(&pixel.0);
                }
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
        path.display()
    );

    Ok(frames)
}
