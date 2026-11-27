//! Plain-text output: the grid without colour.

use crate::render::grid::Grid;

/// Renders `grid` as one line per row, `cols` characters each.
///
/// Colour and background are dropped, which makes the result the monochrome rendering.
#[must_use]
pub fn to_text(grid: &Grid) -> String {
    let cols = grid.cols() as usize;
    let cells = grid.cells();
    let mut out = String::new();
    for row in 0..grid.rows() as usize {
        for cell in &cells[row * cols..row * cols + cols] {
            out.push(cell.ch);
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_text;
    use crate::render::ansi::ColorMode;
    use crate::render::grid::{Cell, Grid, Rgb};

    fn cell(ch: char) -> Cell {
        Cell {
            ch,
            fg: Rgb { r: 1, g: 2, b: 3 },
            bg: None,
        }
    }

    #[test]
    fn rows_keep_the_grid_shape() {
        let grid = Grid::new(3, 2, ColorMode::Mono, vec![cell('a'); 6]);
        assert_eq!(to_text(&grid), "aaa\naaa\n");
    }

    #[test]
    fn a_grid_without_columns_is_an_empty_line_per_row() {
        assert_eq!(
            to_text(&Grid::new(0, 2, ColorMode::Mono, Vec::new())),
            "\n\n"
        );
    }

    #[test]
    fn colour_is_ignored() {
        let coloured = Grid::new(
            1,
            1,
            ColorMode::TrueColor,
            vec![Cell {
                ch: '#',
                fg: Rgb {
                    r: 200,
                    g: 100,
                    b: 50,
                },
                bg: Some(Rgb { r: 0, g: 0, b: 0 }),
            }],
        );
        assert_eq!(to_text(&coloured), "#\n");
    }
}
