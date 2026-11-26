//! Conversion of decoder output into the RGBA layout used by [`crate::RgbaImage`].

/// Expands tightly packed samples into RGBA, inferring the layout from `channels`.
///
/// Supported layouts are 1 (gray), 2 (gray + alpha), 3 (RGB) and 4 (RGBA); anything else is
/// clamped into that range, so a decoder reporting an odd sample count cannot panic here.
pub(crate) fn to_rgba(samples: &[u8], channels: usize) -> Vec<u8> {
    let channels = channels.clamp(1, 4);
    let mut rgba = Vec::with_capacity(samples.len() / channels * 4);
    for pixel in samples.chunks_exact(channels) {
        match channels {
            1 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], 0xFF]),
            2 => rgba.extend_from_slice(&[pixel[0], pixel[0], pixel[0], pixel[1]]),
            3 => rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 0xFF]),
            _ => rgba.extend_from_slice(&pixel[..4]),
        }
    }
    rgba
}

/// Narrows 16-bit samples to 8 bits, rounding to the nearest representable value.
pub(crate) fn narrow_u16(samples: &[u16]) -> Vec<u8> {
    samples
        .iter()
        .map(|&sample| u8::try_from((u32::from(sample) + 128) / 257).unwrap_or(u8::MAX))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{narrow_u16, to_rgba};

    #[test]
    fn gray_becomes_opaque_rgba() {
        assert_eq!(to_rgba(&[7, 9], 1), vec![7, 7, 7, 255, 9, 9, 9, 255]);
    }

    #[test]
    fn gray_alpha_keeps_transparency() {
        assert_eq!(to_rgba(&[7, 128], 2), vec![7, 7, 7, 128]);
    }

    #[test]
    fn rgb_becomes_opaque_rgba() {
        assert_eq!(to_rgba(&[1, 2, 3], 3), vec![1, 2, 3, 255]);
    }

    #[test]
    fn rgba_is_passed_through() {
        assert_eq!(to_rgba(&[1, 2, 3, 4], 4), vec![1, 2, 3, 4]);
    }

    #[test]
    fn trailing_partial_pixel_is_dropped() {
        assert_eq!(to_rgba(&[1, 2, 3, 4], 3), vec![1, 2, 3, 255]);
        assert_eq!(to_rgba(&[], 4), Vec::<u8>::new());
    }

    #[test]
    fn out_of_range_channel_counts_are_clamped() {
        // Zero channels would slice a one-byte pixel to four bytes; clamping keeps it panic-free.
        assert_eq!(to_rgba(&[5], 0), vec![5, 5, 5, 255]);
        assert_eq!(to_rgba(&[1, 2, 3, 4], 9), vec![1, 2, 3, 4]);
    }

    #[test]
    fn sixteen_bit_samples_are_narrowed() {
        assert_eq!(narrow_u16(&[0, 257, 32_768, 65_535]), vec![0, 1, 128, 255]);
    }
}
