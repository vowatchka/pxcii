//! Sampling of the source image down to the character grid.

use crate::image::RgbaImage;
use crate::render::grid::Rgb;

/// Height of the character grid for a `cols`-wide picture of `width` x `height` pixels.
///
/// `cell_aspect` is the width of one character divided by its height, so a square image stays
/// square on screen. The result is never below one row, and a non-finite aspect cannot escape.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub(crate) fn rows(cols: u32, width: u32, height: u32, cell_aspect: f32) -> u32 {
    let exact = cols as f32 * (height as f32 / width as f32) * cell_aspect;
    let rounded = exact.round();
    if rounded >= 1.0 {
        rounded as u32
    } else {
        1
    }
}

/// Averages the source area under every grid cell; the result holds `cols * rows` colours.
pub(crate) fn colors(image: &RgbaImage, cols: u32, rows: u32) -> Vec<Rgb> {
    let (src_w, src_h) = (image.width() as usize, image.height() as usize);
    let (across, down) = (cols as usize, rows as usize);
    let raw = image.as_raw();
    let mut out = Vec::with_capacity(across * down);
    for row in 0..down {
        let (y0, y1) = span(row, down, src_h);
        for col in 0..across {
            let (x0, x1) = span(col, across, src_w);
            let mut sum = [0f32; 3];
            let mut count = 0f32;
            for y in y0..y1 {
                for x in x0..x1 {
                    let offset = (y * src_w + x) * 4;
                    for (total, &value) in sum.iter_mut().zip(&raw[offset..offset + 3]) {
                        *total += f32::from(value);
                    }
                    count += 1.0;
                }
            }
            out.push(Rgb {
                r: mean(sum[0], count),
                g: mean(sum[1], count),
                b: mean(sum[2], count),
            });
        }
    }
    out
}

/// Half-open source range `[start, end)` covered by destination index `index`.
fn span(index: usize, destination: usize, source: usize) -> (usize, usize) {
    let start = index * source / destination;
    let end = ((index + 1) * source / destination).max(start + 1);
    (start, end)
}

/// Rounds an averaged channel; a cell always covers at least one source pixel.
fn mean(total: f32, count: f32) -> u8 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (total / count).round().clamp(0.0, 255.0) as u8;
    value
}

#[cfg(test)]
mod tests {
    use super::{colors, mean, rows};
    use crate::image::RgbaImage;
    use crate::render::grid::Rgb;

    fn solid(width: u32, height: u32, pixel: [u8; 4]) -> RgbaImage {
        let data = pixel
            .iter()
            .copied()
            .cycle()
            .take(width as usize * height as usize * 4)
            .collect();
        RgbaImage::new(width, height, data).expect("valid buffer")
    }

    #[test]
    fn rows_follow_the_aspect_ratio() {
        assert_eq!(rows(80, 100, 100, 0.5), 40);
        assert_eq!(rows(80, 100, 50, 0.5), 20);
        assert_eq!(rows(80, 100, 50, 1.0), 40);
    }

    #[test]
    fn a_flat_picture_still_gets_one_row() {
        assert_eq!(rows(10, 100, 1, 0.5), 1);
    }

    #[test]
    fn a_non_finite_aspect_keeps_one_row() {
        assert_eq!(rows(10, 100, 100, f32::NAN), 1);
        assert_eq!(rows(10, 100, 100, -4.0), 1);
    }

    #[test]
    fn every_cell_gets_the_average_colour_of_its_area() {
        let data = vec![
            0, 0, 0, 255, //
            100, 40, 20, 255, //
            200, 100, 50, 255, //
            60, 60, 60, 255,
        ];
        let image = RgbaImage::new(2, 2, data).expect("valid buffer");
        let grid = colors(&image, 1, 1);
        assert_eq!(
            grid,
            vec![Rgb {
                r: 90,
                g: 50,
                b: 33
            }]
        );
    }

    #[test]
    fn a_solid_image_keeps_its_colour() {
        let image = solid(16, 16, [9, 8, 7, 255]);
        assert!(colors(&image, 4, 4)
            .iter()
            .all(|&rgb| rgb == Rgb { r: 9, g: 8, b: 7 }));
        assert_eq!(colors(&image, 4, 4).len(), 16);
    }

    #[test]
    fn a_grid_without_columns_has_no_cells() {
        assert_eq!(colors(&solid(4, 4, [1, 2, 3, 255]), 0, 2), Vec::new());
    }

    #[test]
    fn a_channel_average_is_clamped_and_rounded() {
        assert_eq!(mean(255.0, 1.0), 255);
        assert_eq!(mean(0.4, 1.0), 0);
        assert_eq!(mean(300.0, 2.0), 150);
    }
}
