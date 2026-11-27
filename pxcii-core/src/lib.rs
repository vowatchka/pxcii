//! `pxcii` core library: image decoding and (from stage 2 on) encoding.
//!
//! The crate never touches the filesystem, paths or environment variables: it only works on
//! in-memory buffers. File I/O lives in the `pxcii` binary and in the Python wrapper.

mod convert;
mod decode;
mod encode;
mod error;
mod format;
mod image;
mod pixels;
mod render;
mod resize;

pub use convert::{convert, Converted};
pub use decode::{decode, decode_with_hint, MAX_PIXELS};
pub use encode::{encode, EncodeOptions, Encoded, Warning};
pub use error::PxciiError;
pub use format::Format;
pub use image::RgbaImage;
pub use render::{
    render_bytes, to_ansi, to_text, Cell, ColorMode, Dither, Grid, Ramp, RenderOptions, Rgb,
    DEFAULT_CELL_ASPECT, DEFAULT_COLS,
};
