//! Error type shared by the whole pipeline.

use crate::format::Format;

/// Everything that can go wrong inside the core library.
///
/// The variants carry no decoder internals on purpose: the same error surfaces through the CLI,
/// the desktop window, the web handler and the Python wrapper.
#[derive(Debug, thiserror::Error)]
pub enum PxciiError {
    /// The input matches neither a known signature nor the format hinted by the caller.
    #[error("unsupported image format")]
    UnsupportedFormat,

    /// The decoder rejected the input while reading `format`.
    #[error("corrupt {format} image")]
    CorruptImage {
        /// Format whose decoder rejected the input.
        format: Format,
    },

    /// The image has more pixels than [`crate::MAX_PIXELS`] or no pixels at all.
    #[error("image of {w}x{h} pixels is outside the supported range")]
    DimensionLimit {
        /// Width reported by the file header.
        w: u32,
        /// Height reported by the file header.
        h: u32,
    },

    /// The encoder for `format` failed.
    #[error("failed to encode {format} image")]
    Encode {
        /// Target format of the failed encode.
        format: Format,
    },

    /// The underlying backend reported an I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::PxciiError;
    use crate::format::Format;

    #[test]
    fn messages_name_the_format() {
        assert_eq!(
            PxciiError::UnsupportedFormat.to_string(),
            "unsupported image format"
        );
        assert_eq!(
            PxciiError::CorruptImage {
                format: Format::Png
            }
            .to_string(),
            "corrupt PNG image"
        );
        assert_eq!(
            PxciiError::Encode {
                format: Format::Webp
            }
            .to_string(),
            "failed to encode WebP image"
        );
        assert_eq!(
            PxciiError::DimensionLimit { w: 10, h: 20 }.to_string(),
            "image of 10x20 pixels is outside the supported range"
        );
    }

    #[test]
    fn io_errors_are_wrapped() {
        let error = PxciiError::from(std::io::Error::other("disk on fire"));
        assert_eq!(error.to_string(), "I/O error: disk on fire");
    }

    #[test]
    fn debug_impl_is_available() {
        assert!(format!("{:?}", PxciiError::UnsupportedFormat).contains("UnsupportedFormat"));
    }
}
