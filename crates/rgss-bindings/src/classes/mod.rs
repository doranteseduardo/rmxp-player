mod bitmap;
mod color;
pub mod common;
mod font;
mod rect;
mod sprite;
mod table;
mod tone;
mod viewport;

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
    Ok(())
}
