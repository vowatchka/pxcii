//! Renders the image named on the command line to ANSI pseudographics.
//!
//! ```sh
//! cargo run -p pxcii-core --example render -- fixtures/gradient.png
//! ```

use std::error::Error;

use pxcii_core::{render_bytes, to_ansi, RenderOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let path = std::env::args().nth(1).ok_or("usage: render <image>")?;
    let bytes = std::fs::read(&path)?;
    let grid = render_bytes(&bytes, &RenderOptions::default())?;
    print!("{}", to_ansi(&grid));
    Ok(())
}
