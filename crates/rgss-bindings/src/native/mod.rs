pub(crate) mod bitmap;
pub(crate) mod font;
mod handles;
pub(crate) mod interpreter;
mod module;
pub(crate) mod plane;
pub(crate) mod sprite;
pub(crate) mod tilemap;
mod types;
mod util;
pub(crate) mod viewport;
pub(crate) mod window;

use anyhow::Result;

pub fn init() -> Result<()> {
    module::init()?;
    interpreter::init()?;
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
pub use interpreter::{drain_commands as drain_interpreter_commands, InterpreterCommand};
pub use module::{config_dir, save_dir, set_config_dir, set_save_dir};
pub(crate) use module::{native_module, set_project_root};
pub use plane::{snapshot as plane_snapshot, PlaneData};
pub use sprite::{snapshot as sprite_snapshot, SpriteData};
pub use tilemap::{snapshot as tilemap_snapshot, TilemapData};
pub(crate) use types::{ColorData, RectData, ToneData};
pub(crate) use util::*;
#[allow(unused_imports)]
pub use viewport::{snapshot as viewport_snapshot, ViewportData};
#[allow(unused_imports)]
pub use window::{snapshot as window_snapshot, WindowData};
