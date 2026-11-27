//! Renderer tests: committed golden grids plus the properties the plan asks for.

use crate::encode::{encode, EncodeOptions};
use crate::error::PxciiError;
use crate::format::Format;
use crate::image::RgbaImage;

use super::{render_bytes, to_ansi, to_text, ColorMode, Dither, Ramp, RenderOptions};

const PNG: &[u8] = include_bytes!("../../../fixtures/gradient.png");

/// The six configurations `xtask gen-fixtures` writes golden files for.
fn configurations() -> Vec<(&'static str, RenderOptions, &'static str)> {
    let options = |color, ramp, dither, cell_aspect| RenderOptions {
        cols: 8,
        color,
        ramp,
        dither,
        cell_aspect,
    };
    vec![
        (
            "ascii-truecolor",
            options(ColorMode::TrueColor, Ramp::Ascii, Dither::None, 0.5),
            include_str!("../../../fixtures/golden/grids/ascii-truecolor.txt"),
        ),
        (
            "blocks-mono",
            options(ColorMode::Mono, Ramp::Blocks, Dither::None, 0.5),
            include_str!("../../../fixtures/golden/grids/blocks-mono.txt"),
        ),
        (
            "braille-mono",
            options(ColorMode::Mono, Ramp::Braille, Dither::None, 0.5),
            include_str!("../../../fixtures/golden/grids/braille-mono.txt"),
        ),
        (
            "ascii-floyd-steinberg",
            options(
                ColorMode::TrueColor,
                Ramp::Ascii,
                Dither::FloydSteinberg,
                0.5,
            ),
            include_str!("../../../fixtures/golden/grids/ascii-floyd-steinberg.txt"),
        ),
        (
            "ascii-ordered",
            options(ColorMode::TrueColor, Ramp::Ascii, Dither::Ordered, 0.5),
            include_str!("../../../fixtures/golden/grids/ascii-ordered.txt"),
        ),
        (
            "ascii-aspect-1",
            options(ColorMode::TrueColor, Ramp::Ascii, Dither::None, 1.0),
            include_str!("../../../fixtures/golden/grids/ascii-aspect-1.txt"),
        ),
    ]
}

/// Encodes a synthetic image with the core encoder, so tests need no filesystem access.
fn png(width: u32, height: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) -> Vec<u8> {
    let mut data = Vec::with_capacity(width as usize * height as usize * 4);
    for y in 0..height {
        for x in 0..width {
            data.extend_from_slice(&pixel(x, y));
        }
    }
    let image = RgbaImage::new(width, height, data).expect("buffer matches the size");
    encode(&image, Format::Png, &EncodeOptions::default())
        .expect("PNG encodes")
        .bytes
}

fn grid(png: &[u8], options: &RenderOptions) -> super::Grid {
    render_bytes(png, options).expect("the input decodes")
}

fn unique_characters(text: &str) -> usize {
    let mut characters: Vec<char> = text.chars().filter(|&ch| ch != '\n').collect();
    characters.sort_unstable();
    characters.dedup();
    characters.len()
}

#[test]
fn golden_grids_match_the_renderer() {
    for (name, options, expected) in configurations() {
        assert_eq!(to_ansi(&grid(PNG, &options)), expected, "{name}");
    }
}

#[test]
fn a_single_pixel_becomes_a_single_cell() {
    let grid = grid(
        &png(1, 1, |_, _| [10, 20, 30, 255]),
        &RenderOptions {
            cols: 1,
            ..RenderOptions::default()
        },
    );
    assert_eq!((grid.cols(), grid.rows()), (1, 1));
    assert_eq!(grid.cells().len(), 1);
}

#[test]
fn every_width_gets_at_least_one_row() {
    let flat = png(16, 1, |x, _| [u8::try_from(x).unwrap_or(0) * 16, 0, 0, 255]);
    for cols in 1..=400 {
        let grid = grid(
            &flat,
            &RenderOptions {
                cols,
                ..RenderOptions::default()
            },
        );
        assert!(grid.rows() >= 1, "cols {cols}");
        assert_eq!(grid.cells().len(), cols as usize * grid.rows() as usize);
    }
}

#[test]
fn monochrome_output_has_no_escape_sequences() {
    let grid = grid(
        PNG,
        &RenderOptions {
            color: ColorMode::Mono,
            ..RenderOptions::default()
        },
    );
    let text = to_ansi(&grid);
    assert!(!text.contains('\x1b'), "{text:?}");
    assert_eq!(text, to_text(&grid));
}

#[test]
fn every_text_line_has_exactly_the_grid_width() {
    let grid = grid(
        PNG,
        &RenderOptions {
            cols: 37,
            ramp: Ramp::Blocks,
            dither: Dither::Ordered,
            ..RenderOptions::default()
        },
    );
    let text = to_text(&grid);
    assert_eq!(text.lines().count(), grid.rows() as usize);
    for line in text.lines() {
        assert_eq!(line.chars().count(), 37);
    }
}

#[test]
fn dithering_adds_characters_to_a_narrow_gradient() {
    // A band that never crosses a ramp level without dithering: every cell maps to one glyph.
    let band = png(256, 1, |x, _| {
        let value = 40 + u8::try_from(x % 3).unwrap_or(0);
        [value, value, value, 255]
    });
    let count = |dither| {
        unique_characters(&to_text(&grid(
            &band,
            &RenderOptions {
                cols: 256,
                dither,
                ..RenderOptions::default()
            },
        )))
    };
    let plain = count(Dither::None);
    assert!(count(Dither::FloydSteinberg) > plain, "plain {plain}");
    assert!(count(Dither::Ordered) > plain, "plain {plain}");
}

#[test]
fn braille_sampling_matches_the_dot_table() {
    let expected = [
        '\u{2801}', '\u{2808}', '\u{2802}', '\u{2810}', //
        '\u{2804}', '\u{2820}', '\u{2840}', '\u{2880}',
    ];
    let options = RenderOptions {
        cols: 1,
        color: ColorMode::Mono,
        ramp: Ramp::Braille,
        ..RenderOptions::default()
    };
    for (position, &character) in expected.iter().enumerate() {
        let (dot_x, dot_y) = (position % 2, position / 2);
        let image = png(2, 4, |x, y| {
            if (x as usize, y as usize) == (dot_x, dot_y) {
                [255, 255, 255, 255]
            } else {
                [0, 0, 0, 255]
            }
        });
        let rendered = to_ansi(&grid(&image, &options));
        assert_eq!(rendered, format!("{character}\n"), "dot {position}");
    }
}

#[test]
fn a_wider_cell_stretches_the_grid() {
    let rows = |cell_aspect| {
        grid(
            PNG,
            &RenderOptions {
                cols: 8,
                cell_aspect,
                ..RenderOptions::default()
            },
        )
        .rows()
    };
    assert_eq!(rows(2.0), rows(1.0) * 2);
}

#[test]
fn a_custom_ramp_limits_the_glyphs() {
    let grid = grid(
        PNG,
        &RenderOptions {
            ramp: Ramp::Custom("#-".to_owned()),
            ..RenderOptions::default()
        },
    );
    assert!(grid
        .cells()
        .iter()
        .all(|cell| cell.ch == '#' || cell.ch == '-'));
}

#[test]
fn an_empty_ramp_draws_blanks() {
    let grid = grid(
        PNG,
        &RenderOptions {
            ramp: Ramp::Custom(String::new()),
            ..RenderOptions::default()
        },
    );
    assert!(grid.cells().iter().all(|cell| cell.ch == ' '));
}

#[test]
fn unreadable_input_reports_the_decode_error() {
    let error = render_bytes(b"not an image", &RenderOptions::default())
        .expect_err("garbage cannot be rendered");
    assert!(matches!(error, PxciiError::UnsupportedFormat), "{error:?}");
}
