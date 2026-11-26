//! Decoder tests: one committed fixture per format plus crafted edge cases.

use std::borrow::Cow;
use std::io::Cursor;

use super::{decode, decode_with_hint, exif, MAX_PIXELS};
use crate::error::PxciiError;
use crate::format::Format;
use crate::image::RgbaImage;

const PNG: &[u8] = include_bytes!("../../../fixtures/gradient.png");
const JPEG: &[u8] = include_bytes!("../../../fixtures/gradient.jpg");
const WEBP: &[u8] = include_bytes!("../../../fixtures/gradient.webp");
const GIF: &[u8] = include_bytes!("../../../fixtures/gradient.gif");
const BMP: &[u8] = include_bytes!("../../../fixtures/gradient.bmp");
const TIFF: &[u8] = include_bytes!("../../../fixtures/gradient.tiff");
const ICO: &[u8] = include_bytes!("../../../fixtures/gradient.ico");
const TGA: &[u8] = include_bytes!("../../../fixtures/gradient.tga");
const PNM: &[u8] = include_bytes!("../../../fixtures/gradient.pnm");

const FIXTURES: [(Format, &[u8]); 9] = [
    (Format::Png, PNG),
    (Format::Jpeg, JPEG),
    (Format::Webp, WEBP),
    (Format::Gif, GIF),
    (Format::Bmp, BMP),
    (Format::Tiff, TIFF),
    (Format::Ico, ICO),
    (Format::Tga, TGA),
    (Format::Pnm, PNM),
];

#[test]
fn decodes_every_fixture_to_sixteen_square_rgba() {
    for (format, bytes) in FIXTURES {
        let image = decode(bytes).unwrap_or_else(|error| panic!("{format}: {error}"));
        assert_eq!(image.width(), 16, "{format}");
        assert_eq!(image.height(), 16, "{format}");
        assert_eq!(image.as_raw().len(), 16 * 16 * 4, "{format}");
        assert_eq!(Format::sniff(bytes), Some(format), "{format}");
    }
}

/// One pixel of the gradient written by `xtask gen-fixtures` (see `gen_fixtures::gradient`).
fn gradient_pixel(x: usize, y: usize) -> [u8; 4] {
    [
        u8::try_from(x * 16).expect("fits"),
        u8::try_from(y * 16).expect("fits"),
        u8::try_from((x + y) * 8).expect("fits"),
        u8::try_from(255 - x * 8).expect("fits"),
    ]
}

fn pixel(image: &RgbaImage, x: usize, y: usize) -> [u8; 4] {
    let offset = (y * 16 + x) * 4;
    let raw = image.as_raw();
    [
        raw[offset],
        raw[offset + 1],
        raw[offset + 2],
        raw[offset + 3],
    ]
}

#[test]
fn lossless_fixtures_reproduce_the_gradient_exactly() {
    for (format, bytes) in [
        (Format::Png, PNG),
        (Format::Webp, WEBP),
        (Format::Bmp, BMP),
        (Format::Tiff, TIFF),
        (Format::Ico, ICO),
        (Format::Tga, TGA),
    ] {
        let image = decode(bytes).unwrap_or_else(|error| panic!("{format}: {error}"));
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(
                    pixel(&image, x, y),
                    gradient_pixel(x, y),
                    "{format} pixel at {x},{y}"
                );
            }
        }
    }
}

#[test]
fn alpha_less_fixtures_keep_the_colours_and_gain_opaque_alpha() {
    for (format, bytes) in [(Format::Gif, GIF), (Format::Pnm, PNM)] {
        let image = decode(bytes).unwrap_or_else(|error| panic!("{format}: {error}"));
        for y in 0..16 {
            for x in 0..16 {
                let expected = gradient_pixel(x, y);
                let actual = pixel(&image, x, y);
                assert_eq!(&actual[..3], &expected[..3], "{format} colour at {x},{y}");
                assert_eq!(actual[3], 255, "{format} alpha at {x},{y}");
            }
        }
    }
}

#[test]
fn jpeg_stays_within_the_compression_tolerance() {
    let image = decode(JPEG).expect("fixture decodes");
    for y in 0..16 {
        for x in 0..16 {
            let expected = gradient_pixel(x, y);
            let actual = pixel(&image, x, y);
            for channel in 0..3 {
                assert!(
                    actual[channel].abs_diff(expected[channel]) <= 8,
                    "JPEG channel {channel} at {x},{y}: {} vs {}",
                    actual[channel],
                    expected[channel]
                );
            }
            assert_eq!(actual[3], 255, "JPEG alpha at {x},{y}");
        }
    }
}

#[test]
fn tga_is_also_decoded_with_an_explicit_hint() {
    let image = decode_with_hint(TGA, Some(Format::Tga)).expect("TGA decodes");
    assert_eq!((image.width(), image.height()), (16, 16));
}

#[test]
fn empty_input_is_unsupported() {
    assert!(matches!(decode(&[]), Err(PxciiError::UnsupportedFormat)));
    assert!(matches!(
        decode_with_hint(&[], Some(Format::Png)),
        Err(PxciiError::CorruptImage {
            format: Format::Png
        })
    ));
}

#[test]
fn garbage_without_hint_is_unsupported() {
    assert!(matches!(
        decode(b"hello, world"),
        Err(PxciiError::UnsupportedFormat)
    ));
}

#[test]
fn truncated_signature_is_corrupt() {
    let bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00];
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::CorruptImage {
            format: Format::Png
        })
    ));
}

#[test]
fn hint_is_used_when_the_sniffed_format_fails() {
    // "BM" makes the sniffer report BMP, but the bytes are not a bitmap; the PNG hint is then tried
    // and its error is reported because that is the format the caller asked for.
    let bytes = b"BMxx";
    assert_eq!(Format::sniff(bytes), Some(Format::Bmp));
    assert!(matches!(
        decode_with_hint(bytes, Some(Format::Png)),
        Err(PxciiError::CorruptImage {
            format: Format::Png
        })
    ));
}

#[test]
fn hint_equal_to_the_signature_is_not_a_fallback() {
    // The signature already says PNG, so there is nothing else to try.
    let bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
    assert!(matches!(
        decode_with_hint(&bytes, Some(Format::Png)),
        Err(PxciiError::CorruptImage {
            format: Format::Png
        })
    ));
}

#[test]
fn declared_size_above_the_limit_is_rejected_before_decoding() {
    let bytes = png_header_with_size(20_000, 20_000);
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::DimensionLimit {
            w: 20_000,
            h: 20_000
        })
    ));
    assert!(u64::from(20_000u32) * u64::from(20_000u32) > MAX_PIXELS);
}

#[test]
fn zero_size_is_rejected() {
    let bytes = png_header_with_size(0, 10);
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::DimensionLimit { w: 0, h: 10 })
    ));
}

/// Builds a PNG signature plus enough of an `IHDR` chunk to declare a size.
fn png_header_with_size(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes
}

#[test]
fn exif_orientation_is_applied_for_all_eight_values() {
    let base = decode(JPEG).expect("fixture decodes");
    for value in 1..=8u16 {
        let tiff = orientation_tiff(value);
        let bytes = jpeg_with_app1(JPEG, &tiff);
        let decoded = decode(&bytes).unwrap_or_else(|error| panic!("orientation {value}: {error}"));
        let expected = exif::apply(&base, exif::parse_orientation(&tiff));
        assert_eq!(decoded, expected, "orientation {value}");
    }
}

/// TIFF block with a single `Orientation` entry, little endian.
fn orientation_tiff(value: u16) -> Vec<u8> {
    let mut tiff = Vec::new();
    tiff.extend_from_slice(b"II");
    tiff.extend_from_slice(&42u16.to_le_bytes());
    tiff.extend_from_slice(&8u32.to_le_bytes());
    tiff.extend_from_slice(&1u16.to_le_bytes());
    tiff.extend_from_slice(&0x0112u16.to_le_bytes());
    tiff.extend_from_slice(&3u16.to_le_bytes());
    tiff.extend_from_slice(&1u32.to_le_bytes());
    tiff.extend_from_slice(&value.to_le_bytes());
    tiff.extend_from_slice(&[0, 0]);
    tiff.extend_from_slice(&[0, 0, 0, 0]);
    tiff
}

/// Inserts an `APP1` EXIF segment right after the JPEG start-of-image marker.
fn jpeg_with_app1(jpeg: &[u8], tiff: &[u8]) -> Vec<u8> {
    let mut bytes = jpeg.get(..2).expect("SOI").to_vec();
    let payload_len = 2 + 6 + tiff.len();
    bytes.extend_from_slice(&[0xFF, 0xE1]);
    bytes.extend_from_slice(&u16::try_from(payload_len).expect("small").to_be_bytes());
    bytes.extend_from_slice(b"Exif\0\0");
    bytes.extend_from_slice(tiff);
    bytes.extend_from_slice(jpeg.get(2..).expect("rest"));
    bytes
}

#[test]
fn png_color_types_are_normalised_to_rgba() {
    let gray = encode_png(png::ColorType::Grayscale, &[3, 7, 11, 13]);
    let image = decode(&gray).expect("gray png");
    assert_eq!(
        image.as_raw(),
        &[3, 3, 3, 255, 7, 7, 7, 255, 11, 11, 11, 255, 13, 13, 13, 255]
    );

    let gray_alpha = encode_png(
        png::ColorType::GrayscaleAlpha,
        &[3, 10, 7, 20, 11, 30, 13, 40],
    );
    let image = decode(&gray_alpha).expect("gray+alpha png");
    assert_eq!(image.as_raw()[..4], [3, 3, 3, 10]);

    let rgb = encode_png(
        png::ColorType::Rgb,
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
    );
    let image = decode(&rgb).expect("rgb png");
    assert_eq!(image.as_raw().len(), 16);
    assert_eq!(image.as_raw()[..4], [1, 2, 3, 255]);

    let rgba = encode_png(
        png::ColorType::Rgba,
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    );
    let image = decode(&rgba).expect("rgba png");
    assert_eq!(
        image.as_raw(),
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    );
}

#[test]
fn png_sixteen_bit_samples_are_stripped() {
    let samples: Vec<u8> = [0u16, 65_535, 32_768, 257]
        .iter()
        .flat_map(|value| value.to_be_bytes())
        .collect();
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(&samples).expect("data");
        writer.finish().expect("finish");
    }
    let image = decode(&bytes).expect("16-bit png");
    assert_eq!(image.as_raw()[..4], [0, 0, 0, 255]);
    assert_eq!(image.as_raw()[4..8], [255, 255, 255, 255]);
}

fn encode_png(color: png::ColorType, samples: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 2, 2);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("header");
        writer.write_image_data(samples).expect("data");
        writer.finish().expect("finish");
    }
    bytes
}

#[test]
fn gif_transparency_survives_the_conversion() {
    let bytes = encode_gif(true);
    let image = decode(&bytes).expect("gif");
    assert_eq!((image.width(), image.height()), (2, 2));
    assert_eq!(
        &image.as_raw()[..4],
        &[255, 0, 0, 255],
        "first pixel keeps its colour"
    );
    assert_eq!(image.as_raw()[7], 0, "transparent pixel has zero alpha");
}

#[test]
fn gif_frame_larger_than_the_logical_screen_is_rejected() {
    // The header declares a 1x1 screen, the frame claims 64x64: without the consistency check the
    // decoder would allocate the frame buffer and bypass the pixel limit.
    let mut palette = Vec::new();
    palette.extend_from_slice(&[255, 0, 0, 255]);
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 1, 1, &palette).expect("gif header");
        let frame = gif::Frame {
            width: 64,
            height: 64,
            buffer: Cow::Owned(vec![0; 64 * 64]),
            ..gif::Frame::default()
        };
        encoder.write_frame(&frame).expect("frame");
    }
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::CorruptImage {
            format: Format::Gif
        })
    ));
}

#[test]
fn gif_without_transparency_is_opaque() {
    let image = decode(&encode_gif(false)).expect("gif");
    assert_eq!(image.as_raw()[3], 255);
    assert_eq!(image.as_raw()[7], 255);
}

fn encode_gif(transparent: bool) -> Vec<u8> {
    let mut palette = Vec::new();
    palette.extend_from_slice(&[255, 0, 0, 255]);
    palette.extend_from_slice(&[0, 255, 0, 255]);
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 2, 2, &palette).expect("gif header");
        let frame = gif::Frame {
            width: 2,
            height: 2,
            buffer: Cow::Owned(vec![0, 1, 0, 1]),
            transparent: transparent.then_some(1),
            ..gif::Frame::default()
        };
        encoder.write_frame(&frame).expect("frame");
    }
    bytes
}

#[test]
fn webp_without_alpha_is_expanded() {
    let mut bytes = Vec::new();
    image_webp::WebPEncoder::new(&mut bytes)
        .encode(
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            2,
            2,
            image_webp::ColorType::Rgb8,
        )
        .expect("webp");
    let image = decode(&bytes).expect("webp");
    assert_eq!(
        image.as_raw(),
        &[1, 2, 3, 255, 4, 5, 6, 255, 7, 8, 9, 255, 10, 11, 12, 255]
    );
}

#[test]
fn tiff_sixteen_bit_gray_is_narrowed() {
    let mut bytes = Vec::new();
    {
        let mut encoder =
            tiff::encoder::TiffEncoder::new(Cursor::new(&mut bytes)).expect("tiff header");
        encoder
            .write_image::<tiff::encoder::colortype::Gray16>(2, 1, &[0, 65_535])
            .expect("image");
    }
    let image = decode(&bytes).expect("gray16 tiff");
    assert_eq!(image.as_raw(), &[0, 0, 0, 255, 255, 255, 255, 255]);
}

#[test]
fn tiff_rgb_is_expanded_to_opaque_rgba() {
    let mut bytes = Vec::new();
    {
        let mut encoder =
            tiff::encoder::TiffEncoder::new(Cursor::new(&mut bytes)).expect("tiff header");
        encoder
            .write_image::<tiff::encoder::colortype::RGB8>(1, 1, &[10, 20, 30])
            .expect("image");
    }
    let image = decode(&bytes).expect("rgb tiff");
    assert_eq!(image.as_raw(), &[10, 20, 30, 255]);
}

#[test]
fn tiff_with_an_unsupported_colour_model_is_corrupt() {
    // CMYK is a valid TIFF colour model the core does not convert.
    let bytes = minimal_tiff(5, 4, 8, 1);
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::CorruptImage {
            format: Format::Tiff
        })
    ));

    // Gray with an extra alpha sample is reported as `Multiband` and is not supported either.
    let bytes = minimal_tiff(1, 2, 8, 1);
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::CorruptImage {
            format: Format::Tiff
        })
    ));
}

#[test]
fn tiff_with_a_signed_sample_format_is_corrupt() {
    // 8-bit signed samples decode to `DecodingResult::I8`, which has no RGBA mapping.
    let bytes = minimal_tiff(1, 1, 8, 2);
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::CorruptImage {
            format: Format::Tiff
        })
    ));
}

/// Minimal uncompressed single-pixel TIFF with a configurable sample layout.
fn minimal_tiff(photometric: u16, samples: u16, bits: u16, sample_format: u16) -> Vec<u8> {
    const ENTRIES: usize = 10;
    let sample_count = usize::from(samples);
    let bits_offset = 8 + 2 + ENTRIES * 12 + 4;
    let format_offset = bits_offset + sample_count * 2;
    let data_offset = format_offset + sample_count * 2;
    let data_len = sample_count * usize::from(bits / 8);
    let offset = |value: usize| u32::try_from(value).expect("fits");
    let short_array = |value: usize| {
        if samples > 1 {
            offset(value)
        } else {
            u32::from(bits)
        }
    };

    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"II");
    bytes.extend_from_slice(&42u16.to_le_bytes());
    bytes.extend_from_slice(&8u32.to_le_bytes());
    bytes.extend_from_slice(&u16::try_from(ENTRIES).expect("fits").to_le_bytes());
    let tags: [(u16, u16, u32, u32); ENTRIES] = [
        (256, 4, 1, 1),
        (257, 4, 1, 1),
        (258, 3, u32::from(samples), short_array(bits_offset)),
        (259, 3, 1, 1),
        (262, 3, 1, u32::from(photometric)),
        (273, 4, 1, offset(data_offset)),
        (277, 3, 1, u32::from(samples)),
        (278, 4, 1, 1),
        (279, 4, 1, offset(data_len)),
        (
            339,
            3,
            u32::from(samples),
            if samples > 1 {
                offset(format_offset)
            } else {
                u32::from(sample_format)
            },
        ),
    ];
    for (tag, kind, count, value) in tags {
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&count.to_le_bytes());
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    for _ in 0..sample_count {
        bytes.extend_from_slice(&bits.to_le_bytes());
    }
    for _ in 0..sample_count {
        bytes.extend_from_slice(&sample_format.to_le_bytes());
    }
    bytes.resize(data_offset + data_len, 0);
    bytes
}

#[test]
fn bmp_core_header_is_probe_only_so_decoding_fails_safely() {
    let mut bytes = vec![0u8; 26];
    bytes[..2].copy_from_slice(b"BM");
    bytes[14..18].copy_from_slice(&12u32.to_le_bytes());
    bytes[18..20].copy_from_slice(&2u16.to_le_bytes());
    bytes[20..22].copy_from_slice(&2u16.to_le_bytes());
    assert!(matches!(
        decode(&bytes),
        Err(PxciiError::CorruptImage {
            format: Format::Bmp
        })
    ));
}

#[test]
fn truncated_fixtures_are_corrupt_for_every_format() {
    for (format, bytes) in FIXTURES {
        let truncated = bytes.get(..bytes.len() / 3).expect("non-empty");
        let result = decode_with_hint(truncated, Some(format));
        assert!(
            matches!(
                result,
                Err(PxciiError::CorruptImage { .. } | PxciiError::DimensionLimit { .. })
            ),
            "{format}: {result:?}"
        );
    }
}

#[test]
fn image_type_accessors_are_consistent() {
    let image = decode(PNG).expect("png");
    let copy: RgbaImage = image.clone();
    assert_eq!(image, copy);
    assert_eq!(
        image.width() as usize * 4,
        image.as_raw().len() / image.height() as usize
    );
}

#[test]
fn probe_only_inputs_are_reported_as_corrupt() {
    // PNG whose IHDR checksum no longer matches: the size probe still succeeds, the decoder does not.
    let mut broken_png = PNG.to_vec();
    broken_png[29] ^= 0xFF;
    assert!(matches!(
        decode(&broken_png),
        Err(PxciiError::CorruptImage {
            format: Format::Png
        })
    ));
}
