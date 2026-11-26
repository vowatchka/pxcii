//! Encoder tests: the 9x9 roundtrip matrix, golden bytes, determinism and format limits.

use super::{encode, EncodeOptions, Warning};
use crate::decode::decode;
use crate::error::PxciiError;
use crate::format::Format;
use crate::image::RgbaImage;

const PNG: &[u8] = include_bytes!("../../../fixtures/gradient.png");

const GOLDEN: [(Format, &[u8]); 6] = [
    (
        Format::Png,
        include_bytes!("../../../fixtures/golden/gradient.png"),
    ),
    (
        Format::Webp,
        include_bytes!("../../../fixtures/golden/gradient.webp"),
    ),
    (
        Format::Bmp,
        include_bytes!("../../../fixtures/golden/gradient.bmp"),
    ),
    (
        Format::Tiff,
        include_bytes!("../../../fixtures/golden/gradient.tiff"),
    ),
    (
        Format::Tga,
        include_bytes!("../../../fixtures/golden/gradient.tga"),
    ),
    (
        Format::Pnm,
        include_bytes!("../../../fixtures/golden/gradient.pnm"),
    ),
];

fn options() -> EncodeOptions {
    EncodeOptions::default()
}

fn fixture() -> RgbaImage {
    decode(PNG).expect("fixture decodes")
}

fn solid(width: u32, height: u32, pixel: [u8; 4]) -> RgbaImage {
    let data = pixel
        .iter()
        .copied()
        .cycle()
        .take(width as usize * height as usize * 4)
        .collect();
    RgbaImage::new(width, height, data).expect("valid buffer")
}

/// Pseudo-random pixels from a fixed generator, so no test depends on a clock or a seed file.
fn noise(width: u32, height: u32, alpha: impl Fn(u32) -> u8) -> RgbaImage {
    let mut state = 0x1234_5678_u32;
    let mut data = Vec::new();
    for index in 0..width * height {
        for _ in 0..3 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            data.push((state >> 24) as u8);
        }
        data.push(alpha(index));
    }
    RgbaImage::new(width, height, data).expect("valid buffer")
}

#[test]
fn roundtrip_matrix_keeps_the_size_for_all_81_pairs() {
    for source in Format::ALL {
        let original = decode(
            &encode(&fixture(), source, &options())
                .expect("source encodes")
                .bytes,
        )
        .expect("source decodes");
        for target in Format::ALL {
            let bytes = encode(&original, target, &options())
                .unwrap_or_else(|error| panic!("{source} -> {target}: {error}"))
                .bytes;
            let again =
                decode(&bytes).unwrap_or_else(|error| panic!("{source} -> {target}: {error}"));
            assert_eq!(
                (again.width(), again.height()),
                (original.width(), original.height()),
                "{source} -> {target}"
            );
        }
    }
}

#[test]
fn lossless_targets_reproduce_the_pixels() {
    let original = fixture();
    for format in [Format::Png, Format::Webp, Format::Tga, Format::Ico] {
        let bytes = encode(&original, format, &options())
            .expect("encodes")
            .bytes;
        assert_eq!(decode(&bytes).expect("decodes"), original, "{format}");
    }
}

#[test]
fn formats_without_alpha_store_the_image_on_black() {
    let original = fixture();
    let expected: Vec<u8> = original
        .as_raw()
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let on_black =
                |c: u8| u8::try_from((u32::from(c) * u32::from(p[3]) + 127) / 255).expect("fits");
            [on_black(p[0]), on_black(p[1]), on_black(p[2]), 255]
        })
        .collect();
    for format in [Format::Bmp, Format::Tiff, Format::Pnm] {
        let bytes = encode(&original, format, &options())
            .expect("encodes")
            .bytes;
        assert_eq!(
            decode(&bytes).expect("decodes").as_raw(),
            &expected[..],
            "{format}"
        );
    }
}

#[test]
fn golden_files_match_byte_for_byte() {
    let original = fixture();
    for (format, golden) in GOLDEN {
        let bytes = encode(&original, format, &options())
            .expect("encodes")
            .bytes;
        assert_eq!(bytes, golden, "{format} drifted from fixtures/golden");
    }
}

#[test]
fn encoding_twice_gives_identical_bytes_for_all_formats() {
    let original = fixture();
    for format in Format::ALL {
        let first = encode(&original, format, &options()).expect("first");
        let second = encode(&original, format, &options()).expect("second");
        assert_eq!(first, second, "{format}");
    }
}

#[test]
fn ico_larger_than_256_is_shrunk_with_a_warning() {
    let encoded =
        encode(&solid(300, 300, [10, 20, 30, 255]), Format::Ico, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![Warning::IcoDownscaled]);
    let image = decode(&encoded.bytes).expect("decodes");
    assert_eq!((image.width(), image.height()), (256, 256));
}

#[test]
fn ico_up_to_256_has_no_warning() {
    let encoded =
        encode(&solid(256, 1, [10, 20, 30, 255]), Format::Ico, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![]);
}

#[test]
fn gif_with_thousands_of_colours_is_quantised() {
    let image = noise(100, 50, |_| 255);
    let encoded = encode(&image, Format::Gif, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![Warning::GifQuantized]);
    let decoded = decode(&encoded.bytes).expect("decodes");
    let colours: std::collections::BTreeSet<_> =
        decoded.as_raw().as_chunks::<4>().0.iter().collect();
    assert!(colours.len() <= 256, "{} colours", colours.len());
    assert!(
        colours.len() > 64,
        "{} colours: palette collapsed",
        colours.len()
    );
}

#[test]
fn gif_with_few_colours_is_exact_and_silent() {
    let image = solid(3, 2, [1, 2, 3, 255]);
    let encoded = encode(&image, Format::Gif, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![]);
    assert_eq!(decode(&encoded.bytes).expect("decodes"), image);
}

#[test]
fn gif_keeps_full_transparency_and_warns_about_partial() {
    let mut data = solid(2, 1, [200, 100, 50, 255]).as_raw().to_vec();
    data[7] = 0;
    let clear = RgbaImage::new(2, 1, data.clone()).expect("valid buffer");
    let encoded = encode(&clear, Format::Gif, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![]);
    let decoded = decode(&encoded.bytes).expect("decodes");
    assert_eq!(decoded.as_raw()[3], 255);
    assert_eq!(decoded.as_raw()[7], 0);

    data[3] = 100;
    let partial = RgbaImage::new(2, 1, data).expect("valid buffer");
    let encoded = encode(&partial, Format::Gif, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![Warning::GifQuantized]);
}

#[test]
fn gif_quantisation_with_transparency_reserves_a_slot() {
    let image = noise(64, 64, |index| if index % 7 == 0 { 0 } else { 255 });
    let encoded = encode(&image, Format::Gif, &options()).expect("encodes");
    assert_eq!(encoded.warnings, vec![Warning::GifQuantized]);
    let decoded = decode(&encoded.bytes).expect("decodes");
    assert_eq!(decoded.as_raw()[3], 0, "pixel 0 is transparent");
    assert_eq!(decoded.as_raw()[7], 255, "pixel 1 is opaque");
}

#[test]
fn jpeg_always_warns_and_flags_dropped_alpha() {
    let opaque = encode(&solid(8, 8, [1, 2, 3, 255]), Format::Jpeg, &options()).expect("encodes");
    assert_eq!(opaque.warnings, vec![Warning::JpegArtifacts]);
    let translucent =
        encode(&solid(8, 8, [1, 2, 3, 100]), Format::Jpeg, &options()).expect("encodes");
    assert_eq!(
        translucent.warnings,
        vec![Warning::AlphaDropped, Warning::JpegArtifacts]
    );
}

#[test]
fn alpha_is_reported_for_every_format_that_drops_it() {
    let translucent = solid(4, 4, [9, 9, 9, 10]);
    for format in [Format::Bmp, Format::Tiff, Format::Pnm] {
        let encoded = encode(&translucent, format, &options()).expect("encodes");
        assert_eq!(encoded.warnings, vec![Warning::AlphaDropped], "{format}");
    }
    for format in [Format::Png, Format::Webp, Format::Tga, Format::Ico] {
        let encoded = encode(&translucent, format, &options()).expect("encodes");
        assert_eq!(encoded.warnings, vec![], "{format}");
    }
}

#[test]
fn jpeg_quality_is_clamped_and_changes_the_output() {
    let image = noise(32, 32, |_| 255);
    let best = encode(&image, Format::Jpeg, &options())
        .expect("encodes")
        .bytes;
    let worst = encode(
        &image,
        Format::Jpeg,
        &EncodeOptions {
            quality: 0,
            ..options()
        },
    )
    .expect("quality 0 is clamped to 1")
    .bytes;
    assert!(worst.len() < best.len());
    let over = encode(
        &image,
        Format::Jpeg,
        &EncodeOptions {
            quality: 200,
            ..options()
        },
    )
    .expect("quality 200 is clamped to 100")
    .bytes;
    assert_eq!(over, best);
}

#[test]
fn jpeg_is_written_without_chroma_subsampling() {
    let bytes = encode(&solid(16, 16, [200, 10, 10, 255]), Format::Jpeg, &options())
        .expect("encodes")
        .bytes;
    // SOF0 carries one sampling byte per component; 0x11 everywhere means 4:4:4.
    let sof = bytes
        .windows(2)
        .position(|pair| pair == [0xFF, 0xC0])
        .expect("baseline frame header");
    let components = &bytes[sof + 10..sof + 19];
    assert_eq!([components[1], components[4], components[7]], [0x11; 3]);
}

#[test]
fn lossy_webp_is_rejected() {
    let result = encode(
        &fixture(),
        Format::Webp,
        &EncodeOptions {
            lossless: false,
            ..options()
        },
    );
    assert!(matches!(
        result,
        Err(PxciiError::Encode {
            format: Format::Webp
        })
    ));
}

#[test]
fn lossless_flag_is_ignored_by_other_formats() {
    let lossy = EncodeOptions {
        lossless: false,
        ..options()
    };
    let png = encode(&fixture(), Format::Png, &lossy).expect("encodes");
    assert_eq!(
        png,
        encode(&fixture(), Format::Png, &options()).expect("encodes")
    );
}

#[test]
fn empty_image_is_a_dimension_error() {
    let empty = RgbaImage::new(0, 0, Vec::new()).expect("valid empty buffer");
    for format in Format::ALL {
        assert!(
            matches!(
                encode(&empty, format, &options()),
                Err(PxciiError::DimensionLimit { w: 0, h: 0 })
            ),
            "{format}"
        );
    }
}

#[test]
fn sides_above_65535_are_an_encode_error() {
    let wide = RgbaImage::new(65_536, 1, vec![0; 65_536 * 4]).expect("valid buffer");
    for format in [Format::Jpeg, Format::Gif] {
        assert!(
            matches!(
                encode(&wide, format, &options()),
                Err(PxciiError::Encode { format: f }) if f == format
            ),
            "{format}"
        );
    }
    assert!(matches!(
        encode(&wide, Format::Tga, &options()),
        Err(PxciiError::Encode {
            format: Format::Tga
        })
    ));
}

#[test]
fn webp_above_16384_pixels_per_side_is_an_encode_error() {
    let wide = RgbaImage::new(16_385, 1, vec![0; 16_385 * 4]).expect("valid buffer");
    assert!(matches!(
        encode(&wide, Format::Webp, &options()),
        Err(PxciiError::Encode {
            format: Format::Webp
        })
    ));
}

#[test]
fn pnm_is_a_binary_ppm() {
    let bytes = encode(&fixture(), Format::Pnm, &options())
        .expect("encodes")
        .bytes;
    assert!(bytes.starts_with(b"P6\n16 16\n255\n"));
    assert_eq!(bytes.len(), 13 + 16 * 16 * 3);
}

#[test]
fn defaults_match_the_plan() {
    let defaults = EncodeOptions::default();
    assert_eq!(defaults.quality, 100);
    assert!(defaults.lossless);
    assert!(format!("{:?}", Warning::AlphaDropped).contains("AlphaDropped"));
}
