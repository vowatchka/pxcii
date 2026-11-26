//! In-memory raster image type shared by decoders, renderers and encoders.

/// Raster image: 8 bits per channel, four channels (red, green, blue, alpha) per pixel.
///
/// Rows are stored top to bottom, pixels left to right, without padding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl RgbaImage {
    /// Channels per pixel.
    pub const CHANNELS: u32 = 4;

    /// Builds an image from raw RGBA bytes.
    ///
    /// Returns `None` when `data` does not hold exactly `width * height * 4` bytes.
    #[must_use]
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        let expected = (width as usize).checked_mul(height as usize)?;
        let expected = expected.checked_mul(Self::CHANNELS as usize)?;
        (data.len() == expected).then_some(Self {
            width,
            height,
            data,
        })
    }

    /// Builds an image whose buffer length the caller guarantees to be `width * height * 4`.
    ///
    /// Used by pure layout transforms (EXIF orientation), which always move exactly the bytes they
    /// were given. Decoders go through [`RgbaImage::new`] instead: a decoder disagreeing with its
    /// own header must surface as [`crate::PxciiError::CorruptImage`], not be padded over.
    pub(crate) fn from_parts(width: u32, height: u32, data: Vec<u8>) -> Self {
        debug_assert_eq!(
            data.len(),
            width as usize * height as usize * Self::CHANNELS as usize,
            "transform changed the buffer size"
        );
        Self {
            width,
            height,
            data,
        }
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA bytes, four per pixel.
    #[must_use]
    pub fn as_raw(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::RgbaImage;

    #[test]
    fn accepts_matching_buffer() {
        let image = RgbaImage::new(2, 1, vec![0; 8]).expect("valid buffer");
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 1);
        assert_eq!(image.as_raw().len(), 8);
    }

    #[test]
    fn from_parts_keeps_the_buffer_as_is() {
        let image = RgbaImage::from_parts(1, 1, vec![1, 2, 3, 4]);
        assert_eq!(image.as_raw(), &[1, 2, 3, 4]);
        assert_eq!((image.width(), image.height()), (1, 1));
    }

    #[test]
    fn rejects_short_buffer() {
        assert!(RgbaImage::new(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn rejects_long_buffer() {
        assert!(RgbaImage::new(1, 1, vec![0; 5]).is_none());
    }

    #[test]
    fn clone_and_equality_are_structural() {
        let image = RgbaImage::new(1, 1, vec![1, 2, 3, 4]).expect("valid buffer");
        let copy = image.clone();
        assert_eq!(image, copy);
        assert_ne!(
            image,
            RgbaImage::new(1, 1, vec![1, 2, 3, 5]).expect("valid buffer")
        );
        assert!(format!("{image:?}").contains("RgbaImage"));
    }
}
