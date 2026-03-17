mod bitmap;
mod handles;
mod module;
mod plane;
mod sprite;
mod tilemap;
mod types;
mod util;
mod viewport;
mod window;

use anyhow::Result;

pub fn init() -> Result<()> {
    module::init()?;
    handles::init();
    bitmap::init()?;
    viewport::init()?;
    sprite::init()?;
    window::init()?;
    plane::init()?;
    tilemap::init()?;
    Ok(())
}

pub use bitmap::{create_from_texture, snapshot as bitmap_snapshot, BitmapData};
pub(crate) use handles::HandleStore;
pub(crate) use module::{native_module, set_project_root};
pub use plane::{snapshot as plane_snapshot, PlaneData};
pub use sprite::{snapshot as sprite_snapshot, SpriteData};
pub use tilemap::{snapshot as tilemap_snapshot, TilemapData};
pub(crate) use types::{ColorData, RectData, ToneData};
pub(crate) use util::*;
pub use viewport::snapshot as viewport_snapshot;
#[allow(unused_imports)]
pub use window::{snapshot as window_snapshot, WindowData};
