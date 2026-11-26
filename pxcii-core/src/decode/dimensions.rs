//! Header-only dimension probing.
//!
//! Reading the declared size before decoding lets the core reject decompression bombs without
//! allocating a single pixel buffer.

use std::io::Cursor;

use crate::format::Format;

/// Reads the pixel dimensions declared in the container header.
///
/// Returns `None` for truncated or inconsistent headers; the caller turns that into
/// `CorruptImage`.
pub(crate) fn probe(format: Format, bytes: &[u8]) -> Option<(u32, u32)> {
    match format {
        Format::Png => png(bytes),
        Format::Jpeg => jpeg(bytes),
        Format::Webp => webp(bytes),
        Format::Gif => gif(bytes),
        Format::Bmp => bmp(bytes),
        Format::Tiff => tiff(bytes),
        Format::Ico => ico(bytes),
        Format::Tga => tga(bytes),
        Format::Pnm => pnm(bytes),
    }
}

fn u16_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_be_bytes([slice[0], slice[1]]))
}

fn u16_le_at(bytes: &[u8], offset: usize) -> Option<u16> {
    let slice = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([slice[0], slice[1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn u32_le_at(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn i32_le_at(bytes: &[u8], offset: usize) -> Option<i32> {
    let slice = bytes.get(offset..offset + 4)?;
    Some(i32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// PNG: `IHDR` always follows the 8-byte signature and a 4-byte length, so the size sits at 16.
fn png(bytes: &[u8]) -> Option<(u32, u32)> {
    Some((u32_at(bytes, 16)?, u32_at(bytes, 20)?))
}

/// JPEG: walk the marker segments until a start-of-frame marker reports the size.
fn jpeg(bytes: &[u8]) -> Option<(u32, u32)> {
    // Skip the SOI marker; every following segment starts with 0xFF and a marker byte, and the
    // specification allows any number of 0xFF fill bytes before the marker.
    let mut offset = 2;
    loop {
        if *bytes.get(offset)? != 0xFF {
            return None;
        }
        let mut marker_offset = offset + 1;
        while *bytes.get(marker_offset)? == 0xFF {
            marker_offset += 1;
        }
        let marker = *bytes.get(marker_offset)?;
        match marker {
            // SOF0..SOF15 minus DHT (0xC4), JPG (0xC8) and DAC (0xCC) carry the frame size.
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) => {
                let height = u32::from(u16_at(bytes, marker_offset + 4)?);
                let width = u32::from(u16_at(bytes, marker_offset + 6)?);
                return Some((width, height));
            }
            // Start of scan: no frame header was found before the entropy-coded data.
            0xDA => return None,
            // Standalone markers carry no length field: TEM, the restart markers and EOI.
            0x01 | 0xD0..=0xD9 => offset = marker_offset + 1,
            _ => {
                let length = usize::from(u16_at(bytes, marker_offset + 1)?);
                offset = marker_offset.checked_add(1 + length)?;
            }
        }
    }
}

/// WebP: the canvas size lives in the `VP8X`, `VP8 ` or `VP8L` chunk header.
fn webp(bytes: &[u8]) -> Option<(u32, u32)> {
    match bytes.get(12..16)? {
        b"VP8X" => {
            let width = 1 + u32_at_24_le(bytes, 24)?;
            let height = 1 + u32_at_24_le(bytes, 27)?;
            Some((width, height))
        }
        b"VP8 " => {
            if bytes.get(23..26)? != [0x9D, 0x01, 0x2A] {
                return None;
            }
            let width = u32::from(u16_le_at(bytes, 26)? & 0x3FFF);
            let height = u32::from(u16_le_at(bytes, 28)? & 0x3FFF);
            Some((width, height))
        }
        b"VP8L" => {
            if *bytes.get(20)? != 0x2F {
                return None;
            }
            let bits = u32_le_at(bytes, 21)?;
            Some(((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1))
        }
        _ => None,
    }
}

fn u32_at_24_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let slice = bytes.get(offset..offset + 3)?;
    Some(u32::from(slice[0]) | (u32::from(slice[1]) << 8) | (u32::from(slice[2]) << 16))
}

/// GIF: the logical screen descriptor follows the 6-byte signature.
fn gif(bytes: &[u8]) -> Option<(u32, u32)> {
    Some((
        u32::from(u16_le_at(bytes, 6)?),
        u32::from(u16_le_at(bytes, 8)?),
    ))
}

/// BMP: `BITMAPINFOHEADER` (and later) store 32-bit sizes, `BITMAPCOREHEADER` 16-bit ones.
fn bmp(bytes: &[u8]) -> Option<(u32, u32)> {
    let header_size = u32_le_at(bytes, 14)?;
    if header_size == 12 {
        return Some((
            u32::from(u16_le_at(bytes, 18)?),
            u32::from(u16_le_at(bytes, 20)?),
        ));
    }
    let width = i32_le_at(bytes, 18)?;
    let height = i32_le_at(bytes, 22)?;
    if width <= 0 || height == 0 {
        return None;
    }
    Some((u32::try_from(width).ok()?, height.unsigned_abs()))
}

/// ICO: width and height of the first directory entry, where zero means 256.
fn ico(bytes: &[u8]) -> Option<(u32, u32)> {
    if u16_le_at(bytes, 4)? == 0 {
        return None;
    }
    let width = u32::from(*bytes.get(6)?);
    let height = u32::from(*bytes.get(7)?);
    Some((
        if width == 0 { 256 } else { width },
        if height == 0 { 256 } else { height },
    ))
}

/// TIFF: the directory is parsed by the `tiff` crate, byte order and offsets included.
fn tiff(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut decoder = tiff::decoder::Decoder::new(Cursor::new(bytes)).ok()?;
    decoder.dimensions().ok()
}

/// TGA: width and height sit at fixed offsets of the header.
fn tga(bytes: &[u8]) -> Option<(u32, u32)> {
    Some((
        u32::from(u16_le_at(bytes, 12)?),
        u32::from(u16_le_at(bytes, 14)?),
    ))
}

/// PNM: the header is ASCII: magic, whitespace or `#` comments, width, height.
fn pnm(bytes: &[u8]) -> Option<(u32, u32)> {
    let mut rest = bytes.get(2..)?;
    let width = decimal_token(&mut rest)?;
    let height = decimal_token(&mut rest)?;
    Some((width, height))
}

fn decimal_token(input: &mut &[u8]) -> Option<u32> {
    loop {
        match input.first()? {
            b'#' => {
                let end = input.iter().position(|&byte| byte == b'\n')?;
                *input = &input[end + 1..];
            }
            byte if byte.is_ascii_whitespace() => *input = &input[1..],
            _ => break,
        }
    }
    let end = input
        .iter()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(input.len());
    let token = std::str::from_utf8(input.get(..end)?).ok()?;
    *input = &input[end..];
    token.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{decimal_token, probe};
    use crate::format::Format;

    #[test]
    fn decimal_token_skips_comments_and_whitespace() {
        let mut input: &[u8] = b"# a comment\n 12 34";
        assert_eq!(decimal_token(&mut input), Some(12));
        assert_eq!(decimal_token(&mut input), Some(34));
        assert_eq!(decimal_token(&mut input), None, "no more digits");
    }

    #[test]
    fn decimal_token_rejects_non_numeric() {
        let mut input: &[u8] = b"abc";
        assert_eq!(decimal_token(&mut input), None);
    }

    #[test]
    fn decimal_token_rejects_unterminated_comment() {
        let mut input: &[u8] = b"# never ends";
        assert_eq!(decimal_token(&mut input), None);
    }

    #[test]
    fn truncated_inputs_are_rejected_for_every_format() {
        for format in Format::ALL {
            assert_eq!(probe(format, b""), None, "{format}");
            assert_eq!(probe(format, b"\x00\x01\x02\x03"), None, "{format}");
        }
    }

    #[test]
    fn bmp_core_header_uses_sixteen_bit_dimensions() {
        let mut bytes = vec![0u8; 26];
        bytes[..2].copy_from_slice(b"BM");
        bytes[14..18].copy_from_slice(&12u32.to_le_bytes());
        bytes[18..20].copy_from_slice(&3u16.to_le_bytes());
        bytes[20..22].copy_from_slice(&4u16.to_le_bytes());
        assert_eq!(probe(Format::Bmp, &bytes), Some((3, 4)));
    }

    #[test]
    fn bmp_top_down_bitmap_reports_absolute_height() {
        let mut bytes = vec![0u8; 54];
        bytes[..2].copy_from_slice(b"BM");
        bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&7i32.to_le_bytes());
        bytes[22..26].copy_from_slice(&(-5i32).to_le_bytes());
        assert_eq!(probe(Format::Bmp, &bytes), Some((7, 5)));
    }

    #[test]
    fn bmp_rejects_negative_width_and_zero_height() {
        let mut bytes = vec![0u8; 54];
        bytes[14..18].copy_from_slice(&40u32.to_le_bytes());
        bytes[18..22].copy_from_slice(&(-7i32).to_le_bytes());
        bytes[22..26].copy_from_slice(&5i32.to_le_bytes());
        assert_eq!(probe(Format::Bmp, &bytes), None);

        bytes[18..22].copy_from_slice(&7i32.to_le_bytes());
        bytes[22..26].copy_from_slice(&0i32.to_le_bytes());
        assert_eq!(probe(Format::Bmp, &bytes), None);
    }

    #[test]
    fn ico_zero_entries_is_rejected_and_zero_sizes_mean_256() {
        let mut bytes = vec![0u8; 22];
        bytes[4..6].copy_from_slice(&1u16.to_le_bytes());
        bytes[6] = 0;
        bytes[7] = 0;
        assert_eq!(probe(Format::Ico, &bytes), Some((256, 256)));

        bytes[4..6].copy_from_slice(&0u16.to_le_bytes());
        assert_eq!(probe(Format::Ico, &bytes), None);
    }

    #[test]
    fn jpeg_skips_fill_bytes_and_standalone_markers() {
        let mut fill = vec![0xFF, 0xD8, 0xFF, 0xFF];
        fill.extend_from_slice(&[0xC0, 0x00, 0x11, 0x08, 0x00, 0x10, 0x00, 0x10]);
        assert_eq!(
            probe(Format::Jpeg, &fill),
            Some((16, 16)),
            "fill byte before SOF"
        );

        let mut markers = vec![0xFF, 0xD8];
        markers.extend_from_slice(&[0xFF, 0xD0]); // RST0, no length field
        markers.extend_from_slice(&[0xFF, 0x01]); // TEM, no length field
        markers.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x02]); // APP0 with an empty payload
        markers.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x20, 0x00, 0x30]);
        assert_eq!(probe(Format::Jpeg, &markers), Some((48, 32)));
    }

    #[test]
    fn jpeg_rejects_scan_before_frame_header() {
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x02]);
        assert_eq!(probe(Format::Jpeg, &bytes), None);
    }

    #[test]
    fn jpeg_rejects_broken_marker_stream() {
        let mut bytes = vec![0xFF, 0xD8];
        bytes.extend_from_slice(&[0x12, 0x34, 0x56]);
        assert_eq!(probe(Format::Jpeg, &bytes), None);
    }

    #[test]
    fn webp_rejects_missing_start_code_and_unknown_chunk() {
        let mut bytes = vec![0u8; 40];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WEBP");
        bytes[12..16].copy_from_slice(b"VP8 ");
        assert_eq!(probe(Format::Webp, &bytes), None, "start code is zeroed");

        bytes[23..26].copy_from_slice(&[0x9D, 0x01, 0x2A]);
        bytes[26..28].copy_from_slice(&13u16.to_le_bytes());
        bytes[28..30].copy_from_slice(&9u16.to_le_bytes());
        assert_eq!(probe(Format::Webp, &bytes), Some((13, 9)));

        bytes[12..16].copy_from_slice(b"VP8L");
        bytes[20] = 0x00;
        assert_eq!(probe(Format::Webp, &bytes), None, "bad lossless signature");

        bytes[12..16].copy_from_slice(b"JUNK");
        assert_eq!(probe(Format::Webp, &bytes), None);
    }

    #[test]
    fn webp_extended_and_lossless_headers_are_parsed() {
        let mut bytes = vec![0u8; 40];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WEBP");
        bytes[12..16].copy_from_slice(b"VP8X");
        // Canvas width and height minus one, three bytes little-endian each.
        bytes[24..27].copy_from_slice(&[0x0F, 0x00, 0x00]);
        bytes[27..30].copy_from_slice(&[0x07, 0x00, 0x00]);
        assert_eq!(probe(Format::Webp, &bytes), Some((16, 8)));

        bytes[12..16].copy_from_slice(b"VP8L");
        bytes[20] = 0x2F;
        // 14 bits width minus one, then 14 bits height minus one.
        let bits: u32 = (15) | (7 << 14);
        bytes[21..25].copy_from_slice(&bits.to_le_bytes());
        assert_eq!(probe(Format::Webp, &bytes), Some((16, 8)));
    }

    #[test]
    fn pnm_header_with_comment_is_parsed() {
        assert_eq!(probe(Format::Pnm, b"P6\n# hi\n3 2\n255\n"), Some((3, 2)));
    }

    #[test]
    fn tiff_without_directory_is_rejected() {
        assert_eq!(probe(Format::Tiff, b"II*\0\xFF\xFF\xFF\x7F"), None);
    }
}
