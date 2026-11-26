//! Encoding of an [`RgbaImage`] into every supported format.
//!
//! Output is deterministic: the same image, format and options always produce the same bytes.
//! Where a format cannot hold the image as is, the encoder adapts it and reports the loss as a
//! [`Warning`] instead of staying silent.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::io::Cursor;

use image::{ExtendedColorType, ImageEncoder};

use crate::error::PxciiError;
use crate::format::Format;
use crate::image::RgbaImage;
use crate::resize;

/// Longest side an ICO entry can have.
const ICO_MAX_SIDE: u32 = 256;

/// Palette size of a GIF frame.
const GIF_COLOURS: usize = 256;

/// Alpha at or above which a GIF pixel is drawn; GIF has only one fully transparent index.
const GIF_VISIBLE_ALPHA: u8 = 0x80;

/// Knobs of [`encode`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeOptions {
    /// JPEG quality, 1 to 100 (values outside are clamped). Other formats ignore it.
    pub quality: u8,
    /// WebP only: the pure-Rust encoder writes lossless files, so `false` is rejected with
    /// [`PxciiError::Encode`] instead of being silently ignored. Other formats ignore it.
    pub lossless: bool,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            quality: 100,
            lossless: true,
        }
    }
}

/// A limitation of the target format that changed the image on its way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Warning {
    /// ICO holds at most 256x256 pixels; the image was shrunk with a box filter.
    IcoDownscaled,
    /// GIF has a palette of 256 colours and one transparent index: colours were reduced with
    /// error diffusion or partial transparency was thresholded.
    GifQuantized,
    /// JPEG is lossy; PNG keeps the image exactly.
    JpegArtifacts,
    /// The format has no alpha channel here; the image was composited on black.
    AlphaDropped,
}

/// Encoded file plus the warnings collected on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encoded {
    /// File contents.
    pub bytes: Vec<u8>,
    /// Limitations that changed the image, in the order they were found.
    pub warnings: Vec<Warning>,
}

/// Encodes `image` into `format`.
///
/// # Errors
///
/// Returns [`PxciiError::DimensionLimit`] for an image without pixels and
/// [`PxciiError::Encode`] when the format cannot store the image: a side above 65535 pixels
/// (JPEG, GIF, TGA), above 16384 (WebP) or `opts.lossless == false` for WebP.
pub fn encode(
    image: &RgbaImage,
    format: Format,
    opts: &EncodeOptions,
) -> Result<Encoded, PxciiError> {
    if image.width() == 0 || image.height() == 0 {
        return Err(PxciiError::DimensionLimit {
            w: image.width(),
            h: image.height(),
        });
    }
    let mut warnings = Vec::new();
    let has_alpha = pixels(image).iter().any(|pixel| pixel[3] != 0xFF);
    if has_alpha
        && matches!(
            format,
            Format::Jpeg | Format::Bmp | Format::Tiff | Format::Pnm
        )
    {
        warnings.push(Warning::AlphaDropped);
    }
    let bytes = match format {
        Format::Png => png_bytes(image),
        Format::Jpeg => {
            warnings.push(Warning::JpegArtifacts);
            jpeg_bytes(image, opts.quality)
        }
        Format::Webp => webp_bytes(image, opts.lossless),
        Format::Gif => gif_bytes(image, &mut warnings),
        Format::Bmp => bmp_bytes(image),
        Format::Tiff => tiff_bytes(image),
        Format::Ico => ico_bytes(image, &mut warnings),
        Format::Tga => tga_bytes(image),
        Format::Pnm => Ok(pnm_bytes(image)),
    }?;
    Ok(Encoded { bytes, warnings })
}

/// Pixels of `image` as RGBA quadruples.
fn pixels(image: &RgbaImage) -> &[[u8; 4]] {
    image.as_raw().as_chunks::<4>().0
}

/// Maps any backend error to [`PxciiError::Encode`]: the backend's details never leave the core.
fn encode_failure<E>(format: Format) -> impl FnOnce(E) -> PxciiError {
    move |_| PxciiError::Encode { format }
}

/// Width and height as `u16`, which JPEG and GIF headers use.
fn u16_size(image: &RgbaImage, format: Format) -> Result<(u16, u16), PxciiError> {
    match (u16::try_from(image.width()), u16::try_from(image.height())) {
        (Ok(width), Ok(height)) => Ok((width, height)),
        _ => Err(PxciiError::Encode { format }),
    }
}

/// Scales one channel by `alpha / 255`, rounding to nearest: the colour over a black backdrop.
fn on_black(channel: u8, alpha: u8) -> u8 {
    let value = (u32::from(channel) * u32::from(alpha) + 127) / 255;
    u8::try_from(value).unwrap_or(u8::MAX)
}

/// Three bytes per pixel, composited on black.
fn rgb_on_black(image: &RgbaImage) -> Vec<u8> {
    pixels(image)
        .iter()
        .flat_map(|&[red, green, blue, alpha]| {
            [red, green, blue].map(|channel| on_black(channel, alpha))
        })
        .collect()
}

fn png_bytes(image: &RgbaImage) -> Result<Vec<u8>, PxciiError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, image.width(), image.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(encode_failure(Format::Png))?;
        writer
            .write_image_data(image.as_raw())
            .map_err(encode_failure(Format::Png))?;
        writer.finish().map_err(encode_failure(Format::Png))?;
    }
    Ok(bytes)
}

fn jpeg_bytes(image: &RgbaImage, quality: u8) -> Result<Vec<u8>, PxciiError> {
    let (width, height) = u16_size(image, Format::Jpeg)?;
    let mut bytes = Vec::new();
    let mut encoder = jpeg_encoder::Encoder::new(&mut bytes, quality.clamp(1, 100));
    encoder.set_sampling_factor(jpeg_encoder::SamplingFactor::F_1_1);
    encoder
        .encode(
            &rgb_on_black(image),
            width,
            height,
            jpeg_encoder::ColorType::Rgb,
        )
        .map_err(encode_failure(Format::Jpeg))?;
    Ok(bytes)
}

fn webp_bytes(image: &RgbaImage, lossless: bool) -> Result<Vec<u8>, PxciiError> {
    if !lossless {
        return Err(PxciiError::Encode {
            format: Format::Webp,
        });
    }
    let mut bytes = Vec::new();
    image_webp::WebPEncoder::new(&mut bytes)
        .encode(
            image.as_raw(),
            image.width(),
            image.height(),
            image_webp::ColorType::Rgba8,
        )
        .map_err(encode_failure(Format::Webp))?;
    Ok(bytes)
}

fn bmp_bytes(image: &RgbaImage) -> Result<Vec<u8>, PxciiError> {
    let mut bytes = Vec::new();
    image::codecs::bmp::BmpEncoder::new(&mut bytes)
        .encode(
            &rgb_on_black(image),
            image.width(),
            image.height(),
            ExtendedColorType::Rgb8,
        )
        .map_err(encode_failure(Format::Bmp))?;
    Ok(bytes)
}

fn tga_bytes(image: &RgbaImage) -> Result<Vec<u8>, PxciiError> {
    let mut bytes = Vec::new();
    image::codecs::tga::TgaEncoder::new(&mut bytes)
        .encode(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(encode_failure(Format::Tga))?;
    Ok(bytes)
}

fn ico_bytes(image: &RgbaImage, warnings: &mut Vec<Warning>) -> Result<Vec<u8>, PxciiError> {
    let shrunk;
    let image = if image.width() > ICO_MAX_SIDE || image.height() > ICO_MAX_SIDE {
        warnings.push(Warning::IcoDownscaled);
        shrunk = resize::shrink_to_fit(image, ICO_MAX_SIDE);
        &shrunk
    } else {
        image
    };
    let mut bytes = Vec::new();
    image::codecs::ico::IcoEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .map_err(encode_failure(Format::Ico))?;
    Ok(bytes)
}

fn tiff_bytes(image: &RgbaImage) -> Result<Vec<u8>, PxciiError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = tiff::encoder::TiffEncoder::new(Cursor::new(&mut bytes))
            .map_err(encode_failure(Format::Tiff))?
            .with_compression(tiff::encoder::Compression::Deflate(
                tiff::encoder::DeflateLevel::default(),
            ));
        encoder
            .write_image::<tiff::encoder::colortype::RGB8>(
                image.width(),
                image.height(),
                &rgb_on_black(image),
            )
            .map_err(encode_failure(Format::Tiff))?;
    }
    Ok(bytes)
}

/// Binary PPM (P6): the `image` crate would write PAM for RGBA, which is outside P1..P6.
fn pnm_bytes(image: &RgbaImage) -> Vec<u8> {
    let mut bytes = format!("P6\n{} {}\n255\n", image.width(), image.height()).into_bytes();
    bytes.extend_from_slice(&rgb_on_black(image));
    bytes
}

fn is_visible(pixel: [u8; 4]) -> bool {
    pixel[3] >= GIF_VISIBLE_ALPHA
}

fn gif_bytes(image: &RgbaImage, warnings: &mut Vec<Warning>) -> Result<Vec<u8>, PxciiError> {
    let (width, height) = u16_size(image, Format::Gif)?;
    let has_transparent = pixels(image).iter().any(|&pixel| !is_visible(pixel));
    let has_partial = pixels(image)
        .iter()
        .any(|pixel| pixel[3] != 0 && pixel[3] != 0xFF);
    // One palette slot is reserved for the transparent index.
    let capacity = GIF_COLOURS - usize::from(has_transparent);

    // Counting stops one past the capacity, so the scan stays linear for any input.
    let mut unique = BTreeSet::new();
    for pixel in pixels(image).iter().filter(|&&pixel| is_visible(pixel)) {
        unique.insert([pixel[0], pixel[1], pixel[2]]);
        if unique.len() > capacity {
            break;
        }
    }
    let exact = unique.len() <= capacity;
    let (mut palette, indices) = if exact {
        // Every colour fits: nearest-colour lookup is exact, so diffusion carries no error.
        let palette: Vec<[u8; 3]> = unique.into_iter().collect();
        let indices = diffuse(image, &palette, |colour| {
            palette.binary_search(&colour).unwrap_or(0)
        });
        (palette, indices)
    } else {
        let training: Vec<u8> = pixels(image)
            .iter()
            .filter(|&&pixel| is_visible(pixel))
            .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 0xFF])
            .collect();
        let network = color_quant::NeuQuant::new(10, capacity, &training);
        let palette: Vec<[u8; 3]> = network.color_map_rgb().as_chunks::<3>().0.to_vec();
        let indices = diffuse(image, &palette, |colour| {
            network.index_of(&[colour[0], colour[1], colour[2], 0xFF])
        });
        (palette, indices)
    };
    if !exact || has_partial {
        warnings.push(Warning::GifQuantized);
    }
    let transparent = u8::try_from(palette.len()).ok().filter(|_| has_transparent);
    if has_transparent {
        palette.push([0, 0, 0]);
    }

    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, width, height, &palette.concat())
            .map_err(encode_failure(Format::Gif))?;
        let frame = gif::Frame {
            width,
            height,
            buffer: Cow::Owned(indices),
            transparent,
            ..gif::Frame::default()
        };
        encoder
            .write_frame(&frame)
            .map_err(encode_failure(Format::Gif))?;
    }
    Ok(bytes)
}

/// Maps every pixel to a palette index with Floyd–Steinberg error diffusion.
///
/// Errors are integers in 1/16 steps, so the result is identical on every platform. Invisible
/// pixels get the index right after the palette (the transparent slot) and spread no error.
fn diffuse(image: &RgbaImage, palette: &[[u8; 3]], index_of: impl Fn([u8; 3]) -> usize) -> Vec<u8> {
    let width = image.width() as usize;
    let transparent = u8::try_from(palette.len()).unwrap_or(0);
    let mut indices = Vec::with_capacity(width * image.height() as usize);
    // Error of the current and the next row; one spare cell on each side avoids edge checks.
    let mut current = vec![[0i32; 3]; width + 2];
    let mut next = current.clone();
    for row in image.as_raw().chunks_exact(width * 4) {
        for (x, pixel) in row.as_chunks::<4>().0.iter().enumerate() {
            if !is_visible(*pixel) {
                indices.push(transparent);
                continue;
            }
            let carried = current[x + 1];
            let wanted: [u8; 3] = std::array::from_fn(|channel| {
                let value = i32::from(pixel[channel]) + carried[channel] / 16;
                u8::try_from(value.clamp(0, 255)).unwrap_or(0)
            });
            let index = index_of(wanted);
            let chosen = palette[index];
            for channel in 0..3 {
                let error = i32::from(wanted[channel]) - i32::from(chosen[channel]);
                current[x + 2][channel] += error * 7;
                next[x][channel] += error * 3;
                next[x + 1][channel] += error * 5;
                next[x + 2][channel] += error;
            }
            indices.push(u8::try_from(index).unwrap_or(0));
        }
        std::mem::swap(&mut current, &mut next);
        next.fill([0; 3]);
    }
    indices
}

#[cfg(test)]
mod tests;
