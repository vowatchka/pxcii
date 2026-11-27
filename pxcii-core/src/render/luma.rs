//! Perceived brightness: sRGB channels into linear light and back.

use crate::render::grid::Rgb;

/// Turns one 8-bit sRGB channel into linear light.
///
/// The exact sRGB curve needs a fractional power, which is not reproducible across platforms on
/// `f32`; gamma 2.0 keeps the transform to a multiplication, which is exactly rounded by IEEE 754.
fn linear(channel: u8) -> f32 {
    let encoded = f32::from(channel) / 255.0;
    encoded * encoded
}

/// Brightness of `rgb` on the 0..=255 scale, weighted for the human eye.
pub(crate) fn luma(rgb: Rgb) -> u8 {
    let light = 0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b);
    let encoded = light.max(0.0).sqrt();
    // The value is clamped to 0..=255 by construction, so the narrowing cast cannot lose data.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let value = (encoded * 255.0).round().clamp(0.0, 255.0) as u8;
    value
}

#[cfg(test)]
mod tests {
    use super::{linear, luma};
    use crate::render::grid::Rgb;

    const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
    const WHITE: Rgb = Rgb {
        r: 255,
        g: 255,
        b: 255,
    };

    #[test]
    fn ends_are_exact() {
        assert_eq!(luma(BLACK), 0);
        assert_eq!(luma(WHITE), 255);
        assert!((linear(255) - 1.0).abs() < f32::EPSILON);
        assert!((linear(0)).abs() < f32::EPSILON);
    }

    #[test]
    fn green_is_brighter_than_red_and_blue() {
        let red = Rgb { r: 255, g: 0, b: 0 };
        let green = Rgb { r: 0, g: 255, b: 0 };
        let blue = Rgb { r: 0, g: 0, b: 255 };
        assert!(luma(green) > luma(red));
        assert!(luma(red) > luma(blue));
    }

    #[test]
    fn mid_grey_sits_between_the_ends() {
        let grey = Rgb {
            r: 128,
            g: 128,
            b: 128,
        };
        assert!(luma(grey) > 100 && luma(grey) < 160, "{}", luma(grey));
    }
}
