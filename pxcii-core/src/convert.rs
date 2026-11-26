//! One-call conversion between the supported formats.

use crate::decode::decode;
use crate::encode::{encode, EncodeOptions, Encoded};
use crate::error::PxciiError;
use crate::format::Format;

/// Result of [`convert`]: the new file and the warnings of the encoding step.
pub type Converted = Encoded;

/// Decodes `bytes` (format detected from the signature) and encodes the image as `format`.
///
/// # Errors
///
/// Returns the error of [`decode`] or of [`encode`].
pub fn convert(
    bytes: &[u8],
    format: Format,
    opts: &EncodeOptions,
) -> Result<Converted, PxciiError> {
    encode(&decode(bytes)?, format, opts)
}

#[cfg(test)]
mod tests {
    use super::convert;
    use crate::encode::{EncodeOptions, Warning};
    use crate::error::PxciiError;
    use crate::format::Format;

    const PNG: &[u8] = include_bytes!("../../fixtures/gradient.png");

    #[test]
    fn converts_png_to_webp() {
        let converted = convert(PNG, Format::Webp, &EncodeOptions::default()).expect("converts");
        assert_eq!(Format::sniff(&converted.bytes), Some(Format::Webp));
        assert_eq!(converted.warnings, vec![]);
    }

    #[test]
    fn warnings_are_passed_through() {
        let converted = convert(PNG, Format::Jpeg, &EncodeOptions::default()).expect("converts");
        assert_eq!(
            converted.warnings,
            vec![Warning::AlphaDropped, Warning::JpegArtifacts]
        );
    }

    #[test]
    fn decode_errors_are_passed_through() {
        let result = convert(b"not an image", Format::Png, &EncodeOptions::default());
        assert!(matches!(result, Err(PxciiError::UnsupportedFormat)));
    }
}
