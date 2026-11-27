//! Knobs of the character renderer.

use crate::render::ansi::ColorMode;
use crate::render::dither::Dither;
use crate::render::ramp::Ramp;

/// Width of the character grid used when the caller does not choose one.
pub const DEFAULT_COLS: u32 = 80;

/// Width of a terminal cell divided by its height: characters are twice as tall as they are wide.
pub const DEFAULT_CELL_ASPECT: f32 = 0.5;

/// Everything [`crate::render_bytes`] needs besides the image bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    /// Width of the grid in characters; the row count follows from it.
    pub cols: u32,
    /// How the character colour is written out.
    pub color: ColorMode,
    /// Which glyphs stand for brightness.
    pub ramp: Ramp,
    /// How brightness is spread over the available ramp levels.
    pub dither: Dither,
    /// Width of a character cell divided by its height, so the picture keeps its proportions.
    pub cell_aspect: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            cols: DEFAULT_COLS,
            color: ColorMode::TrueColor,
            ramp: Ramp::Ascii,
            dither: Dither::None,
            cell_aspect: DEFAULT_CELL_ASPECT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderOptions, DEFAULT_CELL_ASPECT, DEFAULT_COLS};
    use crate::render::ansi::ColorMode;
    use crate::render::dither::Dither;
    use crate::render::ramp::Ramp;

    #[test]
    fn defaults_describe_a_plain_colour_terminal() {
        let options = RenderOptions::default();
        assert_eq!(options.cols, DEFAULT_COLS);
        assert_eq!(options.color, ColorMode::TrueColor);
        assert_eq!(options.ramp, Ramp::Ascii);
        assert_eq!(options.dither, Dither::None);
        assert!((options.cell_aspect - DEFAULT_CELL_ASPECT).abs() < f32::EPSILON);
    }

    #[test]
    fn options_compare_structurally() {
        let options = RenderOptions::default();
        assert_eq!(options, options.clone());
        assert_ne!(
            options,
            RenderOptions {
                cols: 40,
                ..RenderOptions::default()
            }
        );
        assert!(format!("{options:?}").contains("RenderOptions"));
    }
}
