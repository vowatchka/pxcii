//! Decoding of every supported image format into an [`RgbaImage`].
//!
//! Each backend is wrapped in a small "image or nothing" helper: the header has already been
//! validated by [`dimensions::probe`], so a failure can only mean corrupt pixel data, and every
//! such failure is reported as one and the same [`PxciiError::CorruptImage`].

mod dimensions;
mod exif;

use std::io::Cursor;

use crate::error::PxciiError;
use crate::format::Format;
use crate::image::RgbaImage;
use crate::pixels;

/// Highest number of pixels the core accepts from one input.
///
/// The limit is checked on the dimensions declared by the container header, before any pixel
/// buffer is allocated, so a small file cannot make the process allocate gigabytes.
pub const MAX_PIXELS: u64 = 100_000_000;

/// Decodes `bytes`, detecting the format from its signature.
///
/// Formats without a reliable signature (TGA) should be decoded with [`decode_with_hint`].
///
/// # Errors
///
/// Returns [`PxciiError::UnsupportedFormat`] when the signature is unknown,
/// [`PxciiError::CorruptImage`] when the chosen decoder rejects the data and
/// [`PxciiError::DimensionLimit`] when the header declares zero or too many pixels.
pub fn decode(bytes: &[u8]) -> Result<RgbaImage, PxciiError> {
    decode_with_hint(bytes, None)
}

/// Decodes `bytes`, preferring the sniffed format and falling back to `hint` when it fails.
///
/// The fallback is what makes TGA usable: the format has no signature, so the extension-derived
/// hint from the caller is the only reliable source when the header heuristic guesses wrong.
///
/// # Errors
///
/// Same as [`decode`]; when both the sniffed and the hinted format fail, the error of the hinted
/// format is returned because that is the format the caller asked for.
pub fn decode_with_hint(bytes: &[u8], hint: Option<Format>) -> Result<RgbaImage, PxciiError> {
    let sniffed = Format::sniff(bytes);
    let primary = sniffed.or(hint).ok_or(PxciiError::UnsupportedFormat)?;
    match decode_as(bytes, primary) {
        Ok(image) => Ok(image),
        Err(error) => match hint.filter(|hint| sniffed.is_some_and(|sniffed| sniffed != *hint)) {
            Some(fallback) => decode_as(bytes, fallback),
            None => Err(error),
        },
    }
}

fn decode_as(bytes: &[u8], format: Format) -> Result<RgbaImage, PxciiError> {
    let (width, height) =
        dimensions::probe(format, bytes).ok_or(PxciiError::CorruptImage { format })?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(PxciiError::DimensionLimit {
            w: width,
            h: height,
        });
    }
    let decoded = match format {
        Format::Png => png_image(bytes, width, height),
        Format::Jpeg => jpeg_image(bytes, width, height),
        Format::Webp => webp_image(bytes, width, height),
        Format::Gif => gif_image(bytes, width, height),
        Format::Tiff => tiff_image(bytes, width, height),
        Format::Bmp | Format::Ico | Format::Tga | Format::Pnm => bitmap_family_image(bytes, format),
    };
    decoded.ok_or(PxciiError::CorruptImage { format })
}

fn png_image(bytes: &[u8], width: u32, height: u32) -> Option<RgbaImage> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    // Palette and low bit depths are expanded, 16-bit samples are stripped: output is 8-bit.
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().ok()?;
    // Four bytes per pixel is the widest 8-bit output, so the buffer always fits the frame.
    let mut buffer = vec![0u8; width as usize * height as usize * RgbaImage::CHANNELS as usize];
    let frame = reader.next_frame(&mut buffer).ok()?;
    let samples = &buffer[..frame.buffer_size()];
    let data = pixels::to_rgba(samples, frame.color_type.samples());
    RgbaImage::new(width, height, data)
}

fn jpeg_image(bytes: &[u8], width: u32, height: u32) -> Option<RgbaImage> {
    let mut decoder = zune_jpeg::JpegDecoder::new(Cursor::new(bytes));
    let samples = decoder.decode().ok()?;
    // `zune-jpeg` converts to RGB unless another colour space is requested, so CMYK inputs arrive
    // as three components and never end up in the alpha channel. The assertion keeps a future
    // change of decoder options from silently reinterpreting the buffer.
    debug_assert_ne!(
        decoder.output_colorspace(),
        Some(zune_jpeg::zune_core::colorspace::ColorSpace::RGBA),
        "JPEG output must stay RGB"
    );
    let data = pixels::to_rgba(&samples, 3);
    let image = RgbaImage::new(width, height, data)?;
    // `zune-jpeg` hands over the raw EXIF block but never applies the orientation itself.
    let orientation = decoder.exif().map_or(exif::Orientation::Normal, |exif| {
        exif::parse_orientation(exif)
    });
    Some(exif::apply(&image, orientation))
}

fn webp_image(bytes: &[u8], width: u32, height: u32) -> Option<RgbaImage> {
    let mut decoder = image_webp::WebPDecoder::new(Cursor::new(bytes)).ok()?;
    let channels = if decoder.has_alpha() { 4 } else { 3 };
    let mut buffer = vec![0u8; width as usize * height as usize * channels];
    decoder.read_image(&mut buffer).ok()?;
    RgbaImage::new(width, height, pixels::to_rgba(&buffer, channels))
}

fn gif_image(bytes: &[u8], width: u32, height: u32) -> Option<RgbaImage> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    // The header only bounds the logical screen, so a frame may still claim a far larger canvas.
    // Both the consistency check and a finite buffer ceiling keep such a file from allocating
    // beyond the pixel limit checked by `decode_as`.
    options.check_frame_consistency(true);
    options.set_memory_limit(gif::MemoryLimit::Bytes(std::num::NonZeroU64::new(
        MAX_PIXELS * u64::from(RgbaImage::CHANNELS),
    )?));
    let mut decoder = options.read_info(bytes).ok()?;
    // Only the first frame is kept; animated inputs render as a still image.
    let frame = decoder.read_next_frame().ok()??;

    // The first frame may be smaller than the logical screen and offset inside it.
    let canvas_width = width as usize;
    let canvas_height = height as usize;
    let row_bytes = usize::from(frame.width) * RgbaImage::CHANNELS as usize;
    let left = usize::from(frame.left).min(canvas_width);
    let top = usize::from(frame.top).min(canvas_height);
    let rows = usize::from(frame.height).min(canvas_height - top);
    let span = row_bytes.min((canvas_width - left) * RgbaImage::CHANNELS as usize);
    let mut canvas = vec![0u8; canvas_width * canvas_height * RgbaImage::CHANNELS as usize];
    for row in 0..rows {
        let from = row * row_bytes;
        let to = ((top + row) * canvas_width + left) * RgbaImage::CHANNELS as usize;
        canvas[to..to + span].copy_from_slice(&frame.buffer[from..from + span]);
    }
    RgbaImage::new(width, height, canvas)
}

fn tiff_image(bytes: &[u8], width: u32, height: u32) -> Option<RgbaImage> {
    let mut decoder = tiff::decoder::Decoder::new(Cursor::new(bytes)).ok()?;
    let color = decoder.colortype().ok()?;
    // Gray with extra samples is reported as `Multiband`, so gray+alpha is not supported.
    let channels = match color {
        tiff::ColorType::Gray(_) => Some(1usize),
        tiff::ColorType::RGB(_) => Some(3),
        tiff::ColorType::RGBA(_) => Some(4),
        _ => None,
    }?;
    let samples = match decoder.read_image().ok()? {
        tiff::decoder::DecodingResult::U8(samples) => samples,
        tiff::decoder::DecodingResult::U16(samples) => pixels::narrow_u16(&samples),
        _ => return None,
    };
    RgbaImage::new(width, height, pixels::to_rgba(&samples, channels))
}

/// BMP, ICO, TGA and PNM are decoded by the `image` crate, which normalises them to RGBA.
fn bitmap_family_image(bytes: &[u8], format: Format) -> Option<RgbaImage> {
    let image_format = match format {
        Format::Bmp => image::ImageFormat::Bmp,
        Format::Ico => image::ImageFormat::Ico,
        Format::Tga => image::ImageFormat::Tga,
        // The dispatcher only routes PNM here.
        _ => image::ImageFormat::Pnm,
    };
    let rgba = image::load_from_memory_with_format(bytes, image_format)
        .ok()?
        .to_rgba8();
    let (width, height) = rgba.dimensions();
    RgbaImage::new(width, height, rgba.into_raw())
}

#[cfg(test)]
mod tests;
