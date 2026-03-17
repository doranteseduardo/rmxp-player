mod bitmap;
mod handles;
mod module;
mod sprite;
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
    Ok(())
}

pub use bitmap::snapshot as bitmap_snapshot;
pub(crate) use handles::HandleStore;
pub(crate) use module::native_module;
pub use sprite::snapshot as sprite_snapshot;
pub(crate) use types::{ColorData, RectData, ToneData};
pub(crate) use util::*;
pub use viewport::snapshot as viewport_snapshot;
pub use window::snapshot as window_snapshot;
