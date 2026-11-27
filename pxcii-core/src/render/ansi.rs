//! ANSI output: the grid as text with colour escape sequences.

use std::fmt::Write as _;

use crate::render::grid::{Cell, Grid, Rgb};

/// Colours of the 6x6x6 cube the xterm palette uses.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];

/// Distance between two neighbouring greys of the xterm palette.
const GRAY_STEP: u16 = 10;

/// Escape sequence that ends a styled line.
const RESET: &str = "\x1b[0m";

/// How many colours the output may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// 24-bit colour: `\x1b[38;2;r;g;bm`.
    TrueColor,
    /// The 256-colour xterm palette: `\x1b[38;5;nm`.
    Ansi256,
    /// Characters only, no escape sequences at all.
    Mono,
}

/// Renders `grid` as one line per row, each line ending with the colour reset.
///
/// In [`ColorMode::Mono`] the result carries no escape sequences and is the plain text of
/// [`crate::to_text`].
#[must_use]
pub fn to_ansi(grid: &Grid) -> String {
    let cols = grid.cols() as usize;
    let cells = grid.cells();
    let mut out = String::new();
    for row in 0..grid.rows() as usize {
        for cell in &cells[row * cols..row * cols + cols] {
            push_cell(&mut out, grid.color(), cell);
        }
        if grid.color() != ColorMode::Mono {
            out.push_str(RESET);
        }
        out.push('\n');
    }
    out
}

/// Writes one cell: the colour escape of the mode, then the character.
fn push_cell(out: &mut String, mode: ColorMode, cell: &Cell) {
    match mode {
        ColorMode::Mono => {}
        ColorMode::TrueColor => {
            write!(out, "\x1b[38;2;{};{};{}m", cell.fg.r, cell.fg.g, cell.fg.b)
                .expect("writing to a String cannot fail");
        }
        ColorMode::Ansi256 => {
            write!(out, "\x1b[38;5;{}m", xterm(cell.fg)).expect("writing to a String cannot fail");
        }
    }
    out.push(cell.ch);
}

/// Index of the xterm palette entry closest to `rgb`.
fn xterm(rgb: Rgb) -> u8 {
    let cube = |channel: u8| -> u8 {
        if channel < 48 {
            0
        } else if channel < 115 {
            1
        } else {
            (channel - 35) / 40
        }
    };
    let (r, g, b) = (cube(rgb.r), cube(rgb.g), cube(rgb.b));
    let cube_index = 16 + 36 * r + 6 * g + b;
    let cube_colour = Rgb {
        r: CUBE_LEVELS[usize::from(r)],
        g: CUBE_LEVELS[usize::from(g)],
        b: CUBE_LEVELS[usize::from(b)],
    };

    let average = (u16::from(rgb.r) + u16::from(rgb.g) + u16::from(rgb.b)) / 3;
    let gray_index = if average > 238 {
        23
    } else {
        average.saturating_sub(3) / GRAY_STEP
    };
    let gray = u8::try_from(8 + GRAY_STEP * gray_index).unwrap_or(u8::MAX);
    let gray_colour = Rgb {
        r: gray,
        g: gray,
        b: gray,
    };

    if distance(rgb, gray_colour) < distance(rgb, cube_colour) {
        232 + u8::try_from(gray_index).unwrap_or(0)
    } else {
        cube_index
    }
}

/// Squared euclidean distance between two colours.
fn distance(left: Rgb, right: Rgb) -> i32 {
    let squared = |a: u8, b: u8| {
        let delta = i32::from(a) - i32::from(b);
        delta * delta
    };
    squared(left.r, right.r) + squared(left.g, right.g) + squared(left.b, right.b)
}

#[cfg(test)]
mod tests {
    use super::{distance, to_ansi, xterm, ColorMode};
    use crate::render::grid::{Cell, Grid, Rgb};

    fn grid(color: ColorMode, cells: Vec<Cell>) -> Grid {
        Grid::new(2, 1, color, cells)
    }

    fn cell(ch: char, fg: Rgb) -> Cell {
        Cell { ch, fg, bg: None }
    }

    #[test]
    fn truecolour_writes_the_channel_values() {
        let rgb = Rgb { r: 1, g: 2, b: 3 };
        let text = to_ansi(&grid(
            ColorMode::TrueColor,
            vec![cell('a', rgb), cell('b', rgb)],
        ));
        assert_eq!(text, "\x1b[38;2;1;2;3ma\x1b[38;2;1;2;3mb\x1b[0m\n");
    }

    #[test]
    fn ansi256_writes_a_palette_index() {
        let black = Rgb { r: 0, g: 0, b: 0 };
        let text = to_ansi(&grid(
            ColorMode::Ansi256,
            vec![cell('a', black), cell('b', black)],
        ));
        assert_eq!(text, "\x1b[38;5;16ma\x1b[38;5;16mb\x1b[0m\n");
    }

    #[test]
    fn mono_has_no_escape_sequences() {
        let rgb = Rgb { r: 9, g: 9, b: 9 };
        let text = to_ansi(&grid(ColorMode::Mono, vec![cell('a', rgb), cell('b', rgb)]));
        assert_eq!(text, "ab\n");
    }

    #[test]
    fn a_grid_without_columns_is_one_empty_line_per_row() {
        let empty = Grid::new(0, 1, ColorMode::TrueColor, Vec::new());
        assert_eq!(to_ansi(&empty), "\x1b[0m\n");
        assert_eq!(to_ansi(&Grid::new(0, 1, ColorMode::Mono, Vec::new())), "\n");
    }

    #[test]
    fn the_cube_reaches_its_corners() {
        assert_eq!(xterm(Rgb { r: 0, g: 0, b: 0 }), 16);
        assert_eq!(
            xterm(Rgb {
                r: 255,
                g: 255,
                b: 255
            }),
            231
        );
        assert_eq!(xterm(Rgb { r: 95, g: 0, b: 0 }), 52);
        assert_eq!(
            xterm(Rgb {
                r: 40,
                g: 100,
                b: 200
            }),
            26
        );
    }

    #[test]
    fn a_grey_prefers_the_grey_ramp() {
        let grey = Rgb {
            r: 128,
            g: 128,
            b: 128,
        };
        assert_eq!(xterm(grey), 244);
        assert_eq!(
            xterm(Rgb {
                r: 238,
                g: 238,
                b: 238
            }),
            255
        );
    }

    #[test]
    fn distance_grows_with_the_gap() {
        let black = Rgb { r: 0, g: 0, b: 0 };
        assert_eq!(distance(black, black), 0);
        assert_eq!(distance(black, Rgb { r: 1, g: 0, b: 0 }), 1);
        assert_eq!(distance(black, Rgb { r: 3, g: 4, b: 0 }), 25);
    }
}
