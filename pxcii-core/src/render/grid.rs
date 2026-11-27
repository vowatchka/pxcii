//! The character grid the renderer produces.

use crate::render::ansi::ColorMode;

/// Colour of a character cell, 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

/// One character of the grid with the colour it is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Glyph standing for the brightness of the source area.
    pub ch: char,
    /// Colour of the glyph.
    pub fg: Rgb,
    /// Optional background behind the glyph; the renderer of this stage leaves it empty.
    pub bg: Option<Rgb>,
}

/// Characters laid out row by row, plus the colour mode they were rendered for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    cols: u32,
    rows: u32,
    color: ColorMode,
    cells: Vec<Cell>,
}

impl Grid {
    /// Builds a grid from cells stored row by row.
    pub(crate) fn new(cols: u32, rows: u32, color: ColorMode, cells: Vec<Cell>) -> Self {
        debug_assert_eq!(cells.len(), cols as usize * rows as usize, "cell count");
        Self {
            cols,
            rows,
            color,
            cells,
        }
    }

    /// Width in characters.
    #[must_use]
    pub const fn cols(&self) -> u32 {
        self.cols
    }

    /// Height in characters.
    #[must_use]
    pub const fn rows(&self) -> u32 {
        self.rows
    }

    /// Colour mode the grid was rendered for.
    #[must_use]
    pub const fn color(&self) -> ColorMode {
        self.color
    }

    /// Cells row by row, `cols` of them per row.
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
}

#[cfg(test)]
mod tests {
    use super::{Cell, Grid, Rgb};
    use crate::render::ansi::ColorMode;

    fn cell(ch: char) -> Cell {
        Cell {
            ch,
            fg: Rgb { r: 1, g: 2, b: 3 },
            bg: None,
        }
    }

    #[test]
    fn a_grid_reports_its_shape() {
        let grid = Grid::new(2, 2, ColorMode::Ansi256, vec![cell('a'); 4]);
        assert_eq!(grid.cols(), 2);
        assert_eq!(grid.rows(), 2);
        assert_eq!(grid.color(), ColorMode::Ansi256);
        assert_eq!(grid.cells().len(), 4);
        assert_eq!(grid.cells()[3].ch, 'a');
    }

    #[test]
    fn cells_compare_structurally() {
        assert_eq!(cell('x'), cell('x'));
        assert_ne!(cell('x'), cell('y'));
        let with_background = Cell {
            ch: 'x',
            fg: Rgb { r: 1, g: 2, b: 3 },
            bg: Some(Rgb { r: 4, g: 5, b: 6 }),
        };
        assert_ne!(with_background, cell('x'));
        assert!(format!("{with_background:?}").contains("Cell"));
    }
}
