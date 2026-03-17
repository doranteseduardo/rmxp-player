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
pub use module::{config_dir, save_dir, set_config_dir, set_save_dir};
pub(crate) use module::{native_module, project_root, set_project_root};
pub use plane::{snapshot as plane_snapshot, PlaneData};
pub use sprite::{snapshot as sprite_snapshot, SpriteData};
pub use tilemap::{snapshot as tilemap_snapshot, TilemapData};
pub(crate) use types::{ColorData, RectData, ToneData};
pub(crate) use util::*;
#[allow(unused_imports)]
pub use viewport::{snapshot as viewport_snapshot, ViewportData};
#[allow(unused_imports)]
pub use window::{snapshot as window_snapshot, WindowData};
