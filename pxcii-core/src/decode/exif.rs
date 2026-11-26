//! EXIF orientation of JPEG files.
//!
//! `zune-jpeg` hands back the raw EXIF block but does not apply the orientation tag, so the core
//! parses the tag itself and transforms the decoded buffer. Only tag `0x0112` of the first
//! directory is read: that is the single value every camera writes for this purpose.

use crate::image::RgbaImage;

/// The eight orientations defined by the EXIF specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Orientation {
    /// Tag value 1: no transform.
    Normal,
    /// Tag value 2: mirrored left to right.
    FlipHorizontal,
    /// Tag value 3: rotated by 180 degrees.
    Rotate180,
    /// Tag value 4: mirrored top to bottom.
    FlipVertical,
    /// Tag value 5: mirrored across the main diagonal.
    Transpose,
    /// Tag value 6: rotated by 90 degrees clockwise.
    Rotate90,
    /// Tag value 7: mirrored across the anti-diagonal.
    Transverse,
    /// Tag value 8: rotated by 90 degrees counter-clockwise.
    Rotate270,
}

impl Orientation {
    /// Maps the raw tag value; anything outside 1..=8 is treated as "no transform".
    fn from_tag_value(value: u16) -> Self {
        match value {
            2 => Self::FlipHorizontal,
            3 => Self::Rotate180,
            4 => Self::FlipVertical,
            5 => Self::Transpose,
            6 => Self::Rotate90,
            7 => Self::Transverse,
            8 => Self::Rotate270,
            _ => Self::Normal,
        }
    }

    /// Whether the transform exchanges width and height.
    fn swaps_axes(self) -> bool {
        matches!(
            self,
            Self::Transpose | Self::Rotate90 | Self::Transverse | Self::Rotate270
        )
    }
}

/// Tag number of `Orientation` in the first IFD.
const ORIENTATION_TAG: u16 = 0x0112;
/// EXIF type 3 is `SHORT`.
const TYPE_SHORT: u16 = 3;

/// Reads the orientation from a raw TIFF/EXIF block, falling back to [`Orientation::Normal`].
pub(crate) fn parse_orientation(tiff: &[u8]) -> Orientation {
    orientation_value(tiff).map_or(Orientation::Normal, Orientation::from_tag_value)
}

fn orientation_value(tiff: &[u8]) -> Option<u16> {
    let order = ByteOrder::from_header(tiff.get(..2)?)?;
    if order.u16(tiff, 2)? != 42 {
        return None;
    }
    let ifd_offset = usize::try_from(order.u32(tiff, 4)?).ok()?;
    let entries = order.u16(tiff, ifd_offset)?;
    for index in 0..entries {
        let entry = ifd_offset + 2 + usize::from(index) * 12;
        if order.u16(tiff, entry)? != ORIENTATION_TAG {
            continue;
        }
        if order.u16(tiff, entry + 2)? != TYPE_SHORT || order.u32(tiff, entry + 4)? != 1 {
            return None;
        }
        return order.u16(tiff, entry + 8);
    }
    None
}

/// Byte order marker of a TIFF header.
#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

impl ByteOrder {
    fn from_header(marker: &[u8]) -> Option<Self> {
        match marker {
            b"II" => Some(Self::Little),
            b"MM" => Some(Self::Big),
            _ => None,
        }
    }

    fn u16(self, bytes: &[u8], offset: usize) -> Option<u16> {
        let slice = bytes.get(offset..offset + 2)?;
        let pair = [slice[0], slice[1]];
        Some(match self {
            Self::Little => u16::from_le_bytes(pair),
            Self::Big => u16::from_be_bytes(pair),
        })
    }

    fn u32(self, bytes: &[u8], offset: usize) -> Option<u32> {
        let slice = bytes.get(offset..offset + 4)?;
        let quad = [slice[0], slice[1], slice[2], slice[3]];
        Some(match self {
            Self::Little => u32::from_le_bytes(quad),
            Self::Big => u32::from_be_bytes(quad),
        })
    }
}

/// Applies `orientation` to a decoded image, swapping the axes when the transform requires it.
pub(crate) fn apply(image: &RgbaImage, orientation: Orientation) -> RgbaImage {
    let width = image.width();
    let height = image.height();
    let (target_width, target_height) = if orientation.swaps_axes() {
        (height, width)
    } else {
        (width, height)
    };
    let source = image.as_raw();
    let mut target = vec![0u8; source.len()];
    for y in 0..target_height {
        for x in 0..target_width {
            let (source_x, source_y) = match orientation {
                Orientation::Normal => (x, y),
                Orientation::FlipHorizontal => (width - 1 - x, y),
                Orientation::Rotate180 => (width - 1 - x, height - 1 - y),
                Orientation::FlipVertical => (x, height - 1 - y),
                Orientation::Transpose => (y, x),
                Orientation::Rotate90 => (y, height - 1 - x),
                Orientation::Transverse => (width - 1 - y, height - 1 - x),
                Orientation::Rotate270 => (width - 1 - y, x),
            };
            let from = (source_y as usize * width as usize + source_x as usize) * 4;
            let to = (y as usize * target_width as usize + x as usize) * 4;
            target[to..to + 4].copy_from_slice(&source[from..from + 4]);
        }
    }
    RgbaImage::from_parts(target_width, target_height, target)
}

#[cfg(test)]
mod tests {
    use super::{apply, parse_orientation, Orientation};
    use crate::image::RgbaImage;

    /// Builds a TIFF block holding a single `Orientation` entry.
    fn exif_block(order: [u8; 2], value: u16) -> Vec<u8> {
        let little = order == *b"II";
        let word = |value: u16| {
            if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            }
        };
        let long = |value: u32| {
            if little {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            }
        };
        let mut block = Vec::new();
        block.extend_from_slice(&order);
        block.extend_from_slice(&word(42));
        block.extend_from_slice(&long(8)); // offset of the first IFD
        block.extend_from_slice(&word(1)); // one entry
        block.extend_from_slice(&word(0x0112)); // orientation tag
        block.extend_from_slice(&word(3)); // type SHORT
        block.extend_from_slice(&long(1)); // one value
        block.extend_from_slice(&word(value));
        // Inline values shorter than four bytes are padded, then the next-IFD offset follows.
        block.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        block
    }

    #[test]
    fn parses_all_eight_values_little_endian() {
        let expected = [
            (1, Orientation::Normal),
            (2, Orientation::FlipHorizontal),
            (3, Orientation::Rotate180),
            (4, Orientation::FlipVertical),
            (5, Orientation::Transpose),
            (6, Orientation::Rotate90),
            (7, Orientation::Transverse),
            (8, Orientation::Rotate270),
        ];
        for (value, orientation) in expected {
            assert_eq!(
                parse_orientation(&exif_block(*b"II", value)),
                orientation,
                "value {value}"
            );
        }
    }

    #[test]
    fn parses_big_endian_blocks() {
        assert_eq!(
            parse_orientation(&exif_block(*b"MM", 6)),
            Orientation::Rotate90
        );
    }

    #[test]
    fn unknown_values_and_broken_blocks_mean_no_transform() {
        assert_eq!(
            parse_orientation(&exif_block(*b"II", 9)),
            Orientation::Normal
        );
        assert_eq!(parse_orientation(b"XY*\0\x08\0\0\0"), Orientation::Normal);
        assert_eq!(
            parse_orientation(b"II+\0\x08\0\0\0"),
            Orientation::Normal,
            "bad magic"
        );
        assert_eq!(parse_orientation(b"II"), Orientation::Normal, "truncated");
        assert_eq!(
            parse_orientation(b"II*\0\x08\0\0\0"),
            Orientation::Normal,
            "no entries"
        );
        assert_eq!(parse_orientation(&[]), Orientation::Normal);
    }

    #[test]
    fn missing_tag_means_no_transform() {
        let mut block = exif_block(*b"II", 6);
        // Rewrite the tag number to something else.
        block[10..12].copy_from_slice(&0x010Fu16.to_le_bytes());
        assert_eq!(parse_orientation(&block), Orientation::Normal);
    }

    #[test]
    fn wrong_type_or_count_means_no_transform() {
        let mut block = exif_block(*b"II", 6);
        block[12..14].copy_from_slice(&4u16.to_le_bytes()); // type LONG
        assert_eq!(parse_orientation(&block), Orientation::Normal);

        let mut block = exif_block(*b"II", 6);
        block[14..18].copy_from_slice(&2u32.to_le_bytes()); // count 2
        assert_eq!(parse_orientation(&block), Orientation::Normal);
    }

    /// 2x3 source laid out as `A B / C D / E F`, each pixel tagged by its red channel.
    fn labelled() -> RgbaImage {
        let mut data = Vec::new();
        for label in 1..=6u8 {
            data.extend_from_slice(&[label, 0, 0, 255]);
        }
        RgbaImage::new(2, 3, data).expect("consistent buffer")
    }

    fn labels(image: &RgbaImage) -> Vec<u8> {
        image
            .as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|pixel| pixel[0])
            .collect()
    }

    #[test]
    fn every_orientation_produces_the_expected_layout() {
        // Expectations are written out by hand from the EXIF specification, not derived from
        // `apply`, so a wrong transform table cannot pass.
        let source = labelled();
        let expected: [(Orientation, (u32, u32), [u8; 6]); 8] = [
            (Orientation::Normal, (2, 3), [1, 2, 3, 4, 5, 6]),
            (Orientation::FlipHorizontal, (2, 3), [2, 1, 4, 3, 6, 5]),
            (Orientation::Rotate180, (2, 3), [6, 5, 4, 3, 2, 1]),
            (Orientation::FlipVertical, (2, 3), [5, 6, 3, 4, 1, 2]),
            (Orientation::Transpose, (3, 2), [1, 3, 5, 2, 4, 6]),
            (Orientation::Rotate90, (3, 2), [5, 3, 1, 6, 4, 2]),
            (Orientation::Transverse, (3, 2), [6, 4, 2, 5, 3, 1]),
            (Orientation::Rotate270, (3, 2), [2, 4, 6, 1, 3, 5]),
        ];
        for (orientation, (width, height), layout) in expected {
            let transformed = apply(&source, orientation);
            assert_eq!(
                (transformed.width(), transformed.height()),
                (width, height),
                "{orientation:?}"
            );
            assert_eq!(labels(&transformed), layout, "{orientation:?}");
        }
    }

    #[test]
    fn flip_horizontal_mirrors_rows() {
        let image = RgbaImage::new(2, 1, vec![1, 1, 1, 255, 2, 2, 2, 255]).expect("valid");
        let flipped = apply(&image, Orientation::FlipHorizontal);
        assert_eq!(flipped.as_raw(), &[2, 2, 2, 255, 1, 1, 1, 255]);
    }
}
