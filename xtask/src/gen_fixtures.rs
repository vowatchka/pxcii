//! Generates the committed fixtures: the same 16x16 gradient in all nine formats, plus the golden
//! files written by the `pxcii-core` encoder.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use pxcii_core::{EncodeOptions, Format};

use crate::workspace_paths;

/// Edge length of every fixture, in pixels.
const SIZE: u32 = 16;

/// Formats whose encoder output is byte-for-byte reproducible and therefore has a golden file.
pub(crate) const GOLDEN_FORMATS: [Format; 6] = [
    Format::Png,
    Format::Webp,
    Format::Bmp,
    Format::Tiff,
    Format::Tga,
    Format::Pnm,
];

/// Writes one file per supported format, by default into the committed `fixtures/`.
///
/// `--out <directory>` redirects the output, which is how tests generate into `target/` without
/// touching the committed files. Regeneration stays an explicit command for the same reason:
/// a drifting codec must fail [`tests::committed_fixtures_match_the_generator`], not rewrite the
/// reference inputs on the way. Golden files go to the `golden/` subdirectory.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    write_all(&output_directory(args)?)
}

/// Resolves the output directory: the committed `fixtures/` unless `--out <directory>` is given.
fn output_directory(args: &[String]) -> Result<PathBuf, String> {
    match args {
        [] => Ok(workspace_paths::fixtures_dir()),
        [flag, directory] if flag == "--out" => Ok(PathBuf::from(directory)),
        _ => Err("usage: gen-fixtures [--out <directory>]".to_owned()),
    }
}

fn write_all(directory: &Path) -> Result<(), String> {
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("cannot create {}: {error}", directory.display()))?;
    let source = gradient();
    for format in Format::ALL {
        let path = directory.join(format!("gradient.{}", format.extension()));
        write_fixture(&path, &source, format)?;
    }
    let golden = directory.join("golden");
    std::fs::create_dir_all(&golden)
        .map_err(|error| format!("cannot create {}: {error}", golden.display()))?;
    for format in GOLDEN_FORMATS {
        let path = golden.join(format!("gradient.{}", format.extension()));
        let bytes = golden_bytes(&source, format)?;
        std::fs::write(&path, bytes)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    println!(
        "wrote {} fixtures of {SIZE}x{SIZE} and {} golden files to {}",
        Format::ALL.len(),
        GOLDEN_FORMATS.len(),
        directory.display()
    );
    Ok(())
}

/// Encodes the gradient with the core encoder, which is what the golden files pin down.
pub(crate) fn golden_bytes(source: &image::RgbaImage, format: Format) -> Result<Vec<u8>, String> {
    let (width, height) = source.dimensions();
    let image = pxcii_core::RgbaImage::new(width, height, source.as_raw().clone())
        .expect("an `image` buffer always matches its dimensions");
    pxcii_core::encode(&image, format, &EncodeOptions::default())
        .map(|encoded| encoded.bytes)
        .map_err(|error| format!("cannot encode {format}: {error}"))
}

fn write_fixture(path: &Path, source: &image::RgbaImage, format: Format) -> Result<(), String> {
    let bytes = encode(source, format)?;
    std::fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Encodes `source` with the `image` crate: an encoder independent of the core, so the input
/// fixtures do not move together with the code they are meant to test.
fn encode(source: &image::RgbaImage, format: Format) -> Result<Vec<u8>, String> {
    let image_format = match format {
        Format::Png => image::ImageFormat::Png,
        Format::Jpeg => image::ImageFormat::Jpeg,
        Format::Webp => image::ImageFormat::WebP,
        Format::Gif => image::ImageFormat::Gif,
        Format::Bmp => image::ImageFormat::Bmp,
        Format::Tiff => image::ImageFormat::Tiff,
        Format::Ico => image::ImageFormat::Ico,
        Format::Tga => image::ImageFormat::Tga,
        // The `image` crate writes PAM for RGBA; the fixture is the plain binary PPM the plan asks
        // for, so it is assembled by hand (alpha has no place in P1..P6).
        Format::Pnm => return Ok(encode_ppm(source)),
    };
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(source.clone())
        .write_to(&mut Cursor::new(&mut bytes), image_format)
        .map_err(|error| format!("cannot encode {format}: {error}"))?;
    Ok(bytes)
}

/// Writes a binary PPM (P6): a plain ASCII header followed by three bytes per pixel.
fn encode_ppm(source: &image::RgbaImage) -> Vec<u8> {
    let (width, height) = source.dimensions();
    let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in source.pixels() {
        bytes.extend_from_slice(&pixel.0[..3]);
    }
    bytes
}

/// Builds the fixture: a colour gradient with a horizontal alpha ramp.
pub(crate) fn gradient() -> image::RgbaImage {
    image::RgbaImage::from_fn(SIZE, SIZE, |x, y| {
        image::Rgba([
            u8::try_from(x * 16).expect("x < 16"),
            u8::try_from(y * 16).expect("y < 16"),
            u8::try_from((x + y) * 8).expect("fits"),
            u8::try_from(255 - x * 8).expect("fits"),
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::{
        encode, golden_bytes, gradient, output_directory, run, write_all, GOLDEN_FORMATS, SIZE,
    };
    use pxcii_core::Format;

    #[test]
    fn gradient_is_sixteen_pixels_square_with_alpha() {
        let image = gradient();
        assert_eq!(image.dimensions(), (SIZE, SIZE));
        assert_eq!(image.get_pixel(0, 0).0, [0, 0, 0, 255]);
        assert_eq!(image.get_pixel(15, 0).0, [240, 0, 120, 135]);
        assert_eq!(image.get_pixel(0, 15).0, [0, 240, 120, 255]);
    }

    #[test]
    fn every_format_encodes() {
        let image = gradient();
        for format in Format::ALL {
            let bytes = encode(&image, format).expect("encoding succeeds");
            assert!(!bytes.is_empty(), "{format}");
        }
    }

    #[test]
    fn encoding_is_deterministic() {
        let image = gradient();
        for format in Format::ALL {
            assert_eq!(
                encode(&image, format).expect("first"),
                encode(&image, format).expect("second"),
                "{format}"
            );
        }
    }

    #[test]
    fn committed_fixtures_match_the_generator() {
        // Read-only: a drifting codec must fail the build instead of rewriting the reference
        // inputs on its way to the assertion.
        let source = gradient();
        for format in Format::ALL {
            let name = format!("gradient.{}", format.extension());
            let committed = std::fs::read(crate::workspace_paths::fixtures_dir().join(&name))
                .expect("committed fixture");
            let generated = encode(&source, format).expect("encoding succeeds");
            assert_eq!(committed, generated, "{name} is stale");
        }
    }

    #[test]
    fn committed_golden_files_match_the_core_encoder() {
        let source = gradient();
        for format in GOLDEN_FORMATS {
            let name = format!("gradient.{}", format.extension());
            let committed = std::fs::read(
                crate::workspace_paths::fixtures_dir()
                    .join("golden")
                    .join(&name),
            )
            .expect("committed golden file");
            let generated = golden_bytes(&source, format).expect("encoding succeeds");
            assert_eq!(committed, generated, "golden/{name} is stale");
        }
    }

    #[test]
    fn empty_source_is_reported_by_the_golden_encoder() {
        let error = golden_bytes(&image::RgbaImage::new(0, 0), Format::Png)
            .expect_err("an image without pixels cannot be encoded");
        assert!(error.contains("cannot encode PNG"), "{error}");
    }

    #[test]
    fn run_writes_into_an_explicit_directory() {
        let directory = crate::workspace_paths::workspace_root()
            .join("target")
            .join("gen-fixtures-run");
        let _ = std::fs::remove_dir_all(&directory);
        let explicit = directory.to_string_lossy().into_owned();
        run(&["--out".to_owned(), explicit]).expect("explicit directory is accepted");
        for format in Format::ALL {
            let name = format!("gradient.{}", format.extension());
            assert!(directory.join(&name).is_file(), "{name} is written");
        }
        for format in GOLDEN_FORMATS {
            let name = format!("gradient.{}", format.extension());
            assert!(
                directory.join("golden").join(&name).is_file(),
                "golden/{name}"
            );
        }
    }

    #[test]
    fn default_directory_is_the_committed_one() {
        let directory = output_directory(&[]).expect("default");
        assert_eq!(directory, crate::workspace_paths::fixtures_dir());
    }

    #[test]
    fn unknown_arguments_are_rejected() {
        let error = run(&["--nope".to_owned()]).expect_err("usage error");
        assert!(error.contains("usage: gen-fixtures"), "{error}");
    }

    #[test]
    fn unwritable_directory_is_reported() {
        let blocker = crate::workspace_paths::workspace_root()
            .join("target")
            .join("gen-fixtures-test");
        std::fs::write(&blocker, b"a file, not a directory").expect("write");
        let error = write_all(&blocker.join("nested")).expect_err("cannot create the directory");
        assert!(error.contains("cannot create"), "{error}");
    }

    #[test]
    fn write_failure_is_reported() {
        let directory = crate::workspace_paths::workspace_root()
            .join("target")
            .join("gen-fixtures-blocked");
        let _ = std::fs::remove_dir_all(&directory);
        // A directory in place of the first fixture makes `fs::write` fail.
        std::fs::create_dir_all(directory.join("gradient.png")).expect("blocking directory");
        let error = write_all(&directory).expect_err("a directory cannot be overwritten");
        assert!(error.contains("cannot write"), "{error}");
    }

    #[test]
    fn golden_directory_failure_is_reported() {
        let directory = crate::workspace_paths::workspace_root()
            .join("target")
            .join("gen-fixtures-golden-blocked");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("directory");
        std::fs::write(directory.join("golden"), b"a file, not a directory").expect("write");
        let error = write_all(&directory).expect_err("golden cannot be created");
        assert!(error.contains("cannot create"), "{error}");
    }

    #[test]
    fn golden_write_failure_is_reported() {
        let directory = crate::workspace_paths::workspace_root()
            .join("target")
            .join("gen-fixtures-golden-write");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(directory.join("golden").join("gradient.png")).expect("blocker");
        let error = write_all(&directory).expect_err("a directory cannot be overwritten");
        assert!(error.contains("cannot write"), "{error}");
    }

    #[test]
    fn encoding_an_empty_image_is_reported() {
        let error = encode(&image::RgbaImage::new(0, 0), Format::Png)
            .expect_err("a zero-sized PNG cannot be encoded");
        assert!(error.contains("cannot encode PNG"), "{error}");
    }

    #[test]
    fn ppm_header_matches_the_plan_format() {
        let bytes = encode(&gradient(), Format::Pnm).expect("pnm");
        assert!(bytes.starts_with(b"P6\n16 16\n255\n"), "{:?}", &bytes[..12]);
    }
}
