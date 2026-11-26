//! `pxcii` core library: image decoding and (from stage 2 on) encoding.
//!
//! The crate never touches the filesystem, paths or environment variables: it only works on
//! in-memory buffers. File I/O lives in the `pxcii` binary and in the Python wrapper.

mod decode;
mod error;
mod format;
mod image;
mod pixels;

pub use decode::{decode, decode_with_hint, MAX_PIXELS};
pub use error::PxciiError;
pub use format::Format;
pub use image::RgbaImage;
