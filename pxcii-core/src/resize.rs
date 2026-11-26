//! Box-filter downscale used by the ICO output, which is limited to 256 pixels per side.

use crate::image::RgbaImage;

/// Shrinks `src` so that its longer side becomes exactly `max_side`, keeping the aspect ratio.
///
/// The caller guarantees that at least one side of `src` exceeds `max_side`; the function never
/// upscales in that case. The shorter side is rounded down but never drops below one pixel.
/// Each output pixel is the integer average of the source pixels it covers.
pub(crate) fn shrink_to_fit(src: &RgbaImage, max_side: u32) -> RgbaImage {
    let longest = u64::from(src.width().max(src.height()));
    let scaled = |side: u32| {
        let value = (u64::from(side) * u64::from(max_side) / longest).max(1);
        u32::try_from(value).unwrap_or(max_side)
    };
    box_scale(src, scaled(src.width()), scaled(src.height()))
}

/// Half-open source range `[start, end)` covered by destination index `index`.
fn span(index: usize, destination: usize, source: usize) -> (usize, usize) {
    let start = index * source / destination;
    let end = ((index + 1) * source / destination).max(start + 1);
    (start, end)
}

/// Averages the source area under every destination pixel; `dst_*` never exceed the source size.
fn box_scale(src: &RgbaImage, dst_w: u32, dst_h: u32) -> RgbaImage {
    let (src_w, src_h) = (src.width() as usize, src.height() as usize);
    let (cols, rows) = (dst_w as usize, dst_h as usize);
    let raw = src.as_raw();
    let mut data = Vec::with_capacity(cols * rows * RgbaImage::CHANNELS as usize);
    for row in 0..rows {
        let (y0, y1) = span(row, rows, src_h);
        for col in 0..cols {
            let (x0, x1) = span(col, cols, src_w);
            let mut sum = [0u64; 4];
            for y in y0..y1 {
                for x in x0..x1 {
                    let offset = (y * src_w + x) * 4;
                    for (total, &value) in sum.iter_mut().zip(&raw[offset..offset + 4]) {
                        *total += u64::from(value);
                    }
                }
            }
            let count = ((y1 - y0) * (x1 - x0)) as u64;
            data.extend(sum.map(|total| u8::try_from(total / count).unwrap_or(u8::MAX)));
        }
    }
    RgbaImage::from_parts(dst_w, dst_h, data)
}

#[cfg(test)]
mod tests {
    use super::shrink_to_fit;
    use crate::image::RgbaImage;

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
    fn longer_side_lands_exactly_on_the_limit() {
        let image = shrink_to_fit(&solid(300, 300, [9, 8, 7, 6]), 256);
        assert_eq!((image.width(), image.height()), (256, 256));
        let image = shrink_to_fit(&solid(512, 256, [9, 8, 7, 6]), 256);
        assert_eq!((image.width(), image.height()), (256, 128));
        let image = shrink_to_fit(&solid(100, 400, [9, 8, 7, 6]), 256);
        assert_eq!((image.width(), image.height()), (64, 256));
    }

    #[test]
    fn extreme_aspect_ratio_keeps_one_pixel() {
        let image = shrink_to_fit(&solid(1000, 1, [1, 2, 3, 4]), 256);
        assert_eq!((image.width(), image.height()), (256, 1));
    }

    #[test]
    fn solid_colour_survives_scaling() {
        let image = shrink_to_fit(&solid(300, 300, [100, 150, 200, 255]), 256);
        assert!(image
            .as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [100, 150, 200, 255]));
    }

    #[test]
    fn pixels_are_averaged_per_channel() {
        // 2x2 reduced to 1x1 through a limit of 1: the result is the integer mean of four pixels.
        let data = vec![
            255, 0, 0, 255, //
            0, 255, 0, 255, //
            0, 0, 255, 255, //
            255, 255, 0, 255,
        ];
        let image = shrink_to_fit(&RgbaImage::new(2, 2, data).expect("valid buffer"), 1);
        assert_eq!(image.as_raw(), &[127, 127, 63, 255]);
    }
}
