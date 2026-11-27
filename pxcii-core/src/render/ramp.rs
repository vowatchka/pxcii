//! Glyph ramps: which characters stand for which brightness.

/// Characters from dark to light, as listed in the plan.
pub const ASCII: &str = "@%#*+=-:. ";

/// Block characters, listed in the plan from light to dark.
pub const BLOCKS: &str = "░▒▓█";

/// Dot positions of a braille cell, indexed as the samples are supplied.
const BRAILLE_BITS: [u32; 8] = [0, 3, 1, 4, 2, 5, 6, 7];

/// Code point of the blank braille cell.
const BRAILLE_BASE: u32 = 0x2800;

/// Which character set turns brightness into a glyph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ramp {
    /// The ten-character ASCII ramp.
    Ascii,
    /// Quadrant-free block characters: lighter blocks are brighter.
    Blocks,
    /// Two by four dots per cell, the sharpest of the three.
    Braille,
    /// Any string, read from dark to light like [`Ramp::Ascii`].
    Custom(String),
}

impl Ramp {
    /// Glyphs from dark to light; braille has none because its characters are built from dots.
    pub(crate) fn glyphs(&self) -> Vec<char> {
        match self {
            Ramp::Ascii => ASCII.chars().collect(),
            // The plan lists blocks from light to dark, the renderer indexes ramps from dark.
            Ramp::Blocks => BLOCKS.chars().rev().collect(),
            Ramp::Braille => Vec::new(),
            Ramp::Custom(text) => text.chars().collect(),
        }
    }
}

/// Builds the braille character for a 2x4 block of dots, sampled row by row.
pub(crate) fn braille(samples: [bool; 8]) -> char {
    let mask = samples
        .iter()
        .zip(BRAILLE_BITS)
        .filter(|(on, _)| **on)
        .fold(0, |mask, (_, bit)| mask | (1 << bit));
    char::from_u32(BRAILLE_BASE + mask).unwrap_or(' ')
}

#[cfg(test)]
mod tests {
    use super::{braille, Ramp, ASCII, BLOCKS};

    #[test]
    fn ascii_is_read_dark_to_light() {
        let glyphs = Ramp::Ascii.glyphs();
        assert_eq!(glyphs[0], '@');
        assert_eq!(glyphs[9], ' ');
        assert_eq!(glyphs.len(), ASCII.chars().count());
    }

    #[test]
    fn blocks_are_listed_light_to_dark_but_indexed_dark_first() {
        let glyphs = Ramp::Blocks.glyphs();
        assert_eq!(glyphs[0], '█');
        assert_eq!(glyphs[3], '░');
        assert_eq!(glyphs.len(), BLOCKS.chars().count());
    }

    #[test]
    fn braille_has_no_glyph_string() {
        assert_eq!(Ramp::Braille.glyphs(), Vec::new());
    }

    #[test]
    fn a_custom_ramp_keeps_its_characters() {
        assert_eq!(Ramp::Custom("#+-".to_owned()).glyphs(), vec!['#', '+', '-']);
        assert_eq!(Ramp::Custom(String::new()).glyphs(), Vec::new());
    }

    #[test]
    fn each_dot_maps_to_its_braille_bit() {
        let expected = [
            '\u{2801}', '\u{2808}', '\u{2802}', '\u{2810}', //
            '\u{2804}', '\u{2820}', '\u{2840}', '\u{2880}',
        ];
        for (position, &character) in expected.iter().enumerate() {
            let mut samples = [false; 8];
            samples[position] = true;
            assert_eq!(braille(samples), character, "dot {position}");
        }
    }

    #[test]
    fn empty_and_full_cells_are_the_braille_ends() {
        assert_eq!(braille([false; 8]), '\u{2800}');
        assert_eq!(braille([true; 8]), '\u{28FF}');
    }
}
