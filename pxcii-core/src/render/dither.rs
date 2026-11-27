//! Spreading of the quantisation error over the brightness grid.

/// Bayer 8x8 threshold matrix: every value from 0 to 63 appears exactly once.
const BAYER: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// How brightness is distributed over the levels of the ramp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dither {
    /// Every cell takes its nearest level; bands appear on smooth gradients.
    None,
    /// Integer error diffusion, the classic Floyd–Steinberg kernel.
    FloydSteinberg,
    /// Fixed Bayer 8x8 thresholds, no state carried between cells.
    Ordered,
}

/// Maps every cell of a `cols` x `rows` brightness grid to a ramp level.
///
/// Integer arithmetic only: the result is identical on every platform. Braille is not dithered,
/// so the caller never asks for it here.
pub(crate) fn indices(
    luma: &[u8],
    cols: usize,
    rows: usize,
    levels: usize,
    mode: Dither,
) -> Vec<usize> {
    if cols == 0 || rows == 0 {
        return Vec::new();
    }
    match mode {
        Dither::None => luma
            .iter()
            .map(|&value| quantize(i32::from(value), levels))
            .collect(),
        Dither::FloydSteinberg => floyd_steinberg(luma, cols, levels),
        Dither::Ordered => ordered(luma, cols, levels),
    }
}

/// Highest level of a ramp with `levels` glyphs.
fn max_level(levels: usize) -> i64 {
    i64::try_from(levels.saturating_sub(1)).unwrap_or(i64::MAX / 255)
}

/// Maps brightness to the nearest ramp level.
fn quantize(value: i32, levels: usize) -> usize {
    let top = max_level(levels);
    let scaled = (i64::from(value.clamp(0, 255)) * top + 127) / 255;
    usize::try_from(scaled).unwrap_or(0)
}

/// Brightness a ramp level stands for.
fn level_value(level: usize, levels: usize) -> i32 {
    let top = max_level(levels);
    if top == 0 {
        return 0;
    }
    let level = i64::try_from(level).unwrap_or(0).min(top);
    i32::try_from(level * 255 / top).unwrap_or(0)
}

/// Floyd–Steinberg: the error of each cell is pushed to its neighbours in 1/16 steps.
fn floyd_steinberg(luma: &[u8], cols: usize, levels: usize) -> Vec<usize> {
    let mut out = Vec::with_capacity(luma.len());
    // Error of the current and the next row; one spare cell on each side avoids edge checks.
    let mut current = vec![0i32; cols + 2];
    let mut next = current.clone();
    for row in luma.chunks_exact(cols) {
        for (x, &value) in row.iter().enumerate() {
            let wanted = (i32::from(value) + current[x + 1] / 16).clamp(0, 255);
            let level = quantize(wanted, levels);
            let error = wanted - level_value(level, levels);
            current[x + 2] += error * 7;
            next[x] += error * 3;
            next[x + 1] += error * 5;
            next[x + 2] += error;
            out.push(level);
        }
        std::mem::swap(&mut current, &mut next);
        next.fill(0);
    }
    out
}

/// Ordered dithering: a fixed threshold from the Bayer matrix is added before quantising.
fn ordered(luma: &[u8], cols: usize, levels: usize) -> Vec<usize> {
    let step = level_value(1, levels);
    luma.iter()
        .enumerate()
        .map(|(index, &value)| {
            let threshold = i32::from(BAYER[(index / cols) % 8][(index % cols) % 8]);
            let offset = (threshold * 2 - 63) * step / 128;
            quantize(i32::from(value) + offset, levels)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{indices, level_value, max_level, ordered, quantize, Dither, BAYER};

    #[test]
    fn the_bayer_matrix_holds_every_threshold_once() {
        let mut values: Vec<u8> = BAYER.concat();
        values.sort_unstable();
        assert_eq!(values, (0..64).collect::<Vec<u8>>());
    }

    #[test]
    fn quantisation_picks_the_nearest_level() {
        assert_eq!(quantize(0, 10), 0);
        assert_eq!(quantize(255, 10), 9);
        // 128 is halfway between level 4 (113) and level 5 (142): it rounds up.
        assert_eq!(quantize(128, 10), 5);
        assert_eq!(quantize(300, 10), 9);
        assert_eq!(quantize(-5, 10), 0);
    }

    #[test]
    fn a_single_level_ramp_swallows_every_value() {
        assert_eq!(max_level(1), 0);
        assert_eq!(quantize(200, 1), 0);
        assert_eq!(level_value(3, 1), 0);
        assert_eq!(
            indices(&[0, 128, 255], 3, 1, 1, Dither::FloydSteinberg).len(),
            3
        );
        assert!(indices(&[0, 128, 255], 3, 1, 1, Dither::Ordered)
            .iter()
            .all(|&level| level == 0));
    }

    #[test]
    fn a_level_stands_for_its_brightness() {
        assert_eq!(level_value(0, 10), 0);
        assert_eq!(level_value(9, 10), 255);
        assert_eq!(level_value(1, 10), 28);
        assert_eq!(level_value(99, 10), 255);
    }

    #[test]
    fn without_dithering_a_band_stays_flat() {
        let flat = [120u8; 6];
        let levels = indices(&flat, 3, 2, 10, Dither::None);
        assert_eq!(levels.len(), 6);
        assert!(levels.iter().all(|&level| level == levels[0]));
    }

    #[test]
    fn diffusion_spreads_a_narrow_band_over_more_levels() {
        let band: Vec<u8> = (0..64).map(|value| 100 + value / 8).collect();
        let plain = indices(&band, 8, 8, 10, Dither::None);
        let diffused = indices(&band, 8, 8, 10, Dither::FloydSteinberg);
        assert_eq!(plain.len(), 64);
        assert!(
            unique(&diffused) > unique(&plain),
            "{plain:?} vs {diffused:?}"
        );
    }

    #[test]
    fn ordered_dithering_alternates_within_a_flat_band() {
        let flat = [120u8; 16];
        assert!(unique(&ordered(&flat, 4, 10)) > 1);
    }

    #[test]
    fn a_grid_without_cells_has_no_levels() {
        assert_eq!(indices(&[], 0, 2, 10, Dither::None), Vec::new());
        assert_eq!(indices(&[], 3, 0, 10, Dither::None), Vec::new());
    }

    fn unique(levels: &[usize]) -> usize {
        let mut copy = levels.to_vec();
        copy.sort_unstable();
        copy.dedup();
        copy.len()
    }
}
