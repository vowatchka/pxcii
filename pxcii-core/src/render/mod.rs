//! Rendering of an image into a grid of characters.

mod ansi;
mod dither;
mod grid;
mod luma;
mod options;
mod ramp;
mod resize;
mod text;

#[cfg(test)]
mod tests;

pub use ansi::{to_ansi, ColorMode};
pub use dither::Dither;
pub use grid::{Cell, Grid, Rgb};
pub use options::{RenderOptions, DEFAULT_CELL_ASPECT, DEFAULT_COLS};
pub use ramp::Ramp;
pub use text::to_text;

use crate::error::PxciiError;
use crate::image::RgbaImage;

/// Brightness at which a braille dot is lit.
const BRAILLE_THRESHOLD: u8 = 128;

/// Renders the image in `bytes` as a grid of characters.
///
/// The image is decoded first, so the format is detected exactly as in [`crate::decode`].
/// [`RenderOptions::dither`] spreads brightness over the glyph ramp; braille is thresholded
/// instead, so the setting does not affect it.
///
/// # Errors
///
/// Returns the error of [`crate::decode`] when the input is not a readable image.
pub fn render_bytes(bytes: &[u8], options: &RenderOptions) -> Result<Grid, PxciiError> {
    let image = crate::decode(bytes)?;
    Ok(render_image(&image, options))
}

/// Turns a decoded image into the grid, without touching the filesystem.
fn render_image(image: &RgbaImage, options: &RenderOptions) -> Grid {
    let cols = options.cols;
    let rows = resize::rows(cols, image.width(), image.height(), options.cell_aspect);
    let colours = resize::colors(image, cols, rows);
    let cells = if matches!(options.ramp, Ramp::Braille) {
        braille_cells(image, cols, rows, &colours)
    } else {
        glyph_cells(&colours, cols, rows, options)
    };
    Grid::new(cols, rows, options.color, cells)
}

/// Cells drawn with a ramp of glyphs, one glyph per cell.
fn glyph_cells(
    colours: &[grid::Rgb],
    cols: u32,
    rows: u32,
    options: &RenderOptions,
) -> Vec<grid::Cell> {
    let glyphs = options.ramp.glyphs();
    let levels = glyphs.len().max(1);
    let luma: Vec<u8> = colours.iter().map(|&rgb| luma::luma(rgb)).collect();
    let indices = dither::indices(&luma, cols as usize, rows as usize, levels, options.dither);
    colours
        .iter()
        .zip(indices)
        .map(|(&fg, level)| grid::Cell {
            ch: glyphs.get(level).copied().unwrap_or(' '),
            fg,
            bg: None,
        })
        .collect()
}

/// Cells drawn as braille: every cell samples a 2x4 block of the image.
fn braille_cells(
    image: &RgbaImage,
    cols: u32,
    rows: u32,
    colours: &[grid::Rgb],
) -> Vec<grid::Cell> {
    let dots = resize::colors(image, cols * 2, rows * 4);
    let across = cols as usize;
    let mut cells = Vec::with_capacity(colours.len());
    for row in 0..rows as usize {
        for col in 0..across {
            let mut samples = [false; 8];
            for (index, sample) in samples.iter_mut().enumerate() {
                let x = index % 2;
                let y = index / 2;
                let dot = dots[(row * 4 + y) * across * 2 + col * 2 + x];
                *sample = luma::luma(dot) >= BRAILLE_THRESHOLD;
            }
            cells.push(grid::Cell {
                ch: ramp::braille(samples),
                fg: colours[row * across + col],
                bg: None,
            });
        }
    }
    cells
}
