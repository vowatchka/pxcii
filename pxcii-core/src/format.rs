//! Image format identity: file extensions, MIME types and magic-byte sniffing.

use std::fmt;

/// Image formats supported on input and (from stage 2 on) on output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Format {
    /// Portable Network Graphics.
    Png,
    /// JPEG (JFIF and EXIF variants share the same signature).
    Jpeg,
    /// WebP, lossy and lossless.
    Webp,
    /// GIF, still or animated (only the first frame is decoded).
    Gif,
    /// Windows Bitmap.
    Bmp,
    /// Tagged Image File Format, either byte order.
    Tiff,
    /// Windows icon container.
    Ico,
    /// Truevision TGA, which has no signature at all.
    Tga,
    /// Netpbm family: PBM, PGM and PPM, ASCII and binary.
    Pnm,
}

impl Format {
    /// Every supported format, in the order used by documentation and test matrices.
    pub const ALL: [Self; 9] = [
        Self::Png,
        Self::Jpeg,
        Self::Webp,
        Self::Gif,
        Self::Bmp,
        Self::Tiff,
        Self::Ico,
        Self::Tga,
        Self::Pnm,
    ];

    /// Resolves a format from a file extension, with or without the leading dot and in any case.
    #[must_use]
    pub fn from_ext(ext: &str) -> Option<Self> {
        let ext = ext.strip_prefix('.').unwrap_or(ext);
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpeg),
            "webp" => Some(Self::Webp),
            "gif" => Some(Self::Gif),
            "bmp" => Some(Self::Bmp),
            "tif" | "tiff" => Some(Self::Tiff),
            "ico" => Some(Self::Ico),
            "tga" => Some(Self::Tga),
            "ppm" | "pgm" | "pbm" | "pnm" => Some(Self::Pnm),
            _ => None,
        }
    }

    /// Guesses the format from leading bytes.
    ///
    /// TGA has no signature, so its detection is a heuristic over the 18-byte header; callers
    /// should keep the extension-derived [`Format::from_ext`] as a fallback.
    #[must_use]
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

        if bytes.starts_with(&PNG_SIGNATURE) {
            return Some(Self::Png);
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }
        if bytes.starts_with(b"GIF8") {
            return Some(Self::Gif);
        }
        if bytes.starts_with(b"BM") {
            return Some(Self::Bmp);
        }
        if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
            return Some(Self::Tiff);
        }
        if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            return Some(Self::Webp);
        }
        if bytes.starts_with(&[0x00, 0x00, 0x01, 0x00]) {
            return Some(Self::Ico);
        }
        if matches!(bytes, [b'P', b'1'..=b'6', ..]) {
            return Some(Self::Pnm);
        }
        if bytes
            .first_chunk::<TGA_HEADER_LEN>()
            .is_some_and(looks_like_tga)
        {
            return Some(Self::Tga);
        }
        None
    }

    /// MIME type served by the web mode.
    #[must_use]
    pub const fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::Ico => "image/x-icon",
            Self::Tga => "image/x-tga",
            Self::Pnm => "image/x-portable-anymap",
        }
    }

    /// Canonical file extension, without the dot.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Webp => "webp",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::Ico => "ico",
            Self::Tga => "tga",
            Self::Pnm => "pnm",
        }
    }

    /// Human-readable short name used in messages and documentation tables.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Png => "PNG",
            Self::Jpeg => "JPEG",
            Self::Webp => "WebP",
            Self::Gif => "GIF",
            Self::Bmp => "BMP",
            Self::Tiff => "TIFF",
            Self::Ico => "ICO",
            Self::Tga => "TGA",
            Self::Pnm => "PNM",
        }
    }
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Length of the TGA header the heuristic inspects.
const TGA_HEADER_LEN: usize = 18;

/// Checks the fixed fields of a TGA header: colour-map type, image type and pixel depth.
///
/// A colour-mapped image (types 1 and 9) must declare a palette, otherwise the bytes are not TGA.
fn looks_like_tga(header: &[u8; TGA_HEADER_LEN]) -> bool {
    let [_, colour_map_type, image_type, .., depth, _] = *header;
    let known_type = matches!(image_type, 1 | 2 | 3 | 9 | 10 | 11);
    let known_depth = matches!(depth, 8 | 15 | 16 | 24 | 32);
    let palette_ok = colour_map_type == 1 || !matches!(image_type, 1 | 9);
    known_type && known_depth && palette_ok && colour_map_type <= 1
}

#[cfg(test)]
mod tests {
    use super::{Format, TGA_HEADER_LEN};

    fn tga_header(image_type: u8, colour_map_type: u8, depth: u8) -> Vec<u8> {
        let mut header = vec![0u8; TGA_HEADER_LEN];
        header[1] = colour_map_type;
        header[2] = image_type;
        header[16] = depth;
        header
    }

    #[test]
    fn extensions_with_and_without_dot() {
        assert_eq!(Format::from_ext("png"), Some(Format::Png));
        assert_eq!(Format::from_ext(".png"), Some(Format::Png));
        assert_eq!(Format::from_ext("Png"), Some(Format::Png));
        assert_eq!(Format::from_ext(".JPEG"), Some(Format::Jpeg));
        assert_eq!(Format::from_ext("jpg"), Some(Format::Jpeg));
        assert_eq!(Format::from_ext(".TIF"), Some(Format::Tiff));
        assert_eq!(Format::from_ext("tiff"), Some(Format::Tiff));
        assert_eq!(Format::from_ext("webp"), Some(Format::Webp));
        assert_eq!(Format::from_ext("gif"), Some(Format::Gif));
        assert_eq!(Format::from_ext(".bmp"), Some(Format::Bmp));
        assert_eq!(Format::from_ext("ico"), Some(Format::Ico));
        assert_eq!(Format::from_ext(".TGA"), Some(Format::Tga));
        assert_eq!(Format::from_ext("ppm"), Some(Format::Pnm));
        assert_eq!(Format::from_ext("pgm"), Some(Format::Pnm));
        assert_eq!(Format::from_ext("pbm"), Some(Format::Pnm));
        assert_eq!(Format::from_ext("pnm"), Some(Format::Pnm));
        assert_eq!(Format::from_ext("png2"), None);
        assert_eq!(Format::from_ext(""), None);
        assert_eq!(Format::from_ext("."), None);
        // A path-like string is not an extension; callers pass the extension only.
        assert_eq!(Format::from_ext("photo.png"), None);
    }

    #[test]
    fn extension_and_mime_round_trip() {
        for format in Format::ALL {
            assert_eq!(Format::from_ext(format.extension()), Some(format));
            assert!(format.mime().starts_with("image/"), "{format}");
            assert!(format.mime().contains('/'), "{format}");
            assert!(!format.name().is_empty(), "{format}");
            assert!(!format.extension().starts_with('.'), "{format}");
        }
    }

    #[test]
    fn detects_every_signature() {
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0];
        assert_eq!(Format::sniff(&png), Some(Format::Png));

        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0, 0, 0];
        assert_eq!(Format::sniff(&jpeg), Some(Format::Jpeg));

        assert_eq!(Format::sniff(b"GIF89a\x10\x00\x10\x00"), Some(Format::Gif));
        assert_eq!(Format::sniff(b"GIF87a"), Some(Format::Gif));

        let bmp = b"BM\x36\x00\x00\x00";
        assert_eq!(Format::sniff(bmp), Some(Format::Bmp));

        assert_eq!(Format::sniff(b"II*\0\x08\x00\x00\x00"), Some(Format::Tiff));
        assert_eq!(Format::sniff(b"MM\0*\x00\x00\x00\x08"), Some(Format::Tiff));

        let mut webp = b"RIFF\x24\x00\x00\x00WEBPVP8 ".to_vec();
        assert_eq!(Format::sniff(&webp), Some(Format::Webp));
        webp[8] = b'X';
        assert_eq!(Format::sniff(&webp), None);

        let ico = [0x00, 0x00, 0x01, 0x00, 0x01, 0x00];
        assert_eq!(Format::sniff(&ico), Some(Format::Ico));

        assert_eq!(Format::sniff(b"P6\n2 2\n255\n"), Some(Format::Pnm));
        assert_eq!(Format::sniff(b"P1"), Some(Format::Pnm));
        assert_eq!(Format::sniff(b"P7\n"), None);

        assert_eq!(
            Format::sniff(&tga_header(2, 0, 32)),
            Some(Format::Tga),
            "uncompressed true-colour TGA"
        );
        assert_eq!(
            Format::sniff(&tga_header(10, 0, 24)),
            Some(Format::Tga),
            "RLE true-colour TGA"
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(Format::sniff(b""), None);
        assert_eq!(Format::sniff(b"hello, world"), None);
        assert_eq!(Format::sniff(&[0u8; 32]), None);
        assert_eq!(Format::sniff(b"BMW is not a bitmap"), Some(Format::Bmp));
    }

    #[test]
    fn rejects_truncated_headers() {
        assert_eq!(Format::sniff(&[0x89, b'P', b'N', b'G']), None);
        assert_eq!(Format::sniff(b"RIFF"), None);
        assert_eq!(Format::sniff(b"RIFF\0\0\0\0WEB"), None);
        assert_eq!(Format::sniff(&tga_header(2, 0, 32)[..17]), None);
    }

    #[test]
    fn tga_heuristic_needs_a_palette_for_colour_mapped_images() {
        // A colour-mapped TGA without a palette is not a TGA; these bytes happen to spell the ICO
        // signature, which is exactly why callers keep the extension hint as a fallback.
        assert_eq!(Format::sniff(&tga_header(1, 0, 8)), Some(Format::Ico));
        assert_eq!(Format::sniff(&tga_header(9, 0, 8)), None);
        assert_eq!(Format::sniff(&tga_header(1, 1, 8)), Some(Format::Tga));
        assert_eq!(
            Format::sniff(&tga_header(2, 2, 32)),
            None,
            "bad colour-map type"
        );
        assert_eq!(Format::sniff(&tga_header(2, 0, 7)), None, "bad depth");
        assert_eq!(Format::sniff(&tga_header(4, 0, 32)), None, "bad image type");
        assert_eq!(
            Format::sniff(&tga_header(3, 0, 8)),
            Some(Format::Tga),
            "grayscale"
        );
    }

    #[test]
    fn display_matches_short_name() {
        assert_eq!(Format::Png.to_string(), "PNG");
        assert_eq!(Format::Webp.to_string(), "WebP");
        assert_eq!(Format::Jpeg.to_string(), "JPEG");
        assert_eq!(format!("{}", Format::Pnm), "PNM");
    }
}
