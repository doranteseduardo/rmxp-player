mod bitmap;
mod color;
pub mod common;
mod font;
mod plane;
mod rect;
pub mod sprite;
mod table;
mod tilemap;
mod tone;
mod viewport;
mod window;

use anyhow::Result;

pub fn init() -> Result<()> {
    color::init()?;
    tone::init()?;
    rect::init()?;
    table::init()?;
    font::init()?;
    bitmap::init()?;
    viewport::init()?;
    sprite::init()?;
    plane::init()?;
    window::init()?;
    tilemap::init()?;
    Ok(())
}
