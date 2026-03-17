//! Embedded Ruby (MRI) host for RGSS scripts.

mod graphics;
mod input;
mod kernel;
mod native;
mod scripts;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{
    rb_errinfo, rb_eval_string_protect, rb_obj_as_string, rb_string_value_cstr, ruby_init_stack,
    ruby_setup, ruby_sysinit, VALUE,
};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    path::Path,
    ptr::addr_of_mut,
};
use tracing::{debug, info, warn};

static RUBY_INIT: OnceCell<()> = OnceCell::new();

pub use input::{
    update_input, InputSnapshot, BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_DOWN, BUTTON_LEFT,
    BUTTON_RIGHT, BUTTON_UP,
};
pub use native::{
    bitmap_snapshot, plane_snapshot, sprite_snapshot, tilemap_snapshot, BitmapData, PlaneData,
    SpriteData, TilemapData,
};

pub fn set_project_root(path: &Path) {
    native::set_project_root(path);
}

pub fn sync_graphics_size(width: u32, height: u32) {
    graphics::set_screen_size(width, height);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeSnapshot {
    pub bitmaps: usize,
    pub sprites: usize,
    pub viewports: usize,
    pub windows: usize,
    pub planes: usize,
    pub tilemaps: usize,
}

pub fn native_snapshot() -> NativeSnapshot {
    NativeSnapshot {
        bitmaps: native::bitmap_snapshot().len(),
        sprites: native::sprite_snapshot().len(),
        viewports: native::viewport_snapshot().len(),
        windows: native::window_snapshot().len(),
        planes: native::plane_snapshot().len(),
        tilemaps: native::tilemap_snapshot().len(),
    }
}

pub struct RubyVm {
    booted: bool,
}

pub struct ScriptSection<'a> {
    pub id: i32,
    pub name: &'a str,
    pub source: &'a str,
}

impl RubyVm {
    pub fn new() -> Self {
        Self { booted: false }
    }

    /// Boots the embedded Ruby interpreter (MRI).
    pub fn boot(&mut self) -> Result<()> {
        ensure_ruby()?;
        if !self.booted {
            info!(target: "rgss", "Ruby VM initialised");
            graphics::init()?;
            kernel::init()?;
            input::init()?;
            native::init()?;
            self.booted = true;
            scripts::load(self)?;
        }
        Ok(())
    }

    /// Evaluate a snippet for diagnostics.
    pub fn eval(&self, code: &str) -> Result<()> {
        ensure_ruby()?;
        let script = CString::new(code)?;
        let mut state: c_int = 0;
        unsafe {
            rb_eval_string_protect(script.as_ptr(), &mut state);
            if state != 0 {
                let message = current_exception_message();
                warn!(target: "rgss", %message, "Ruby eval failed");
                return Err(anyhow!("Ruby eval failed: {message}"));
            }
        }
        Ok(())
    }

    pub fn is_booted(&self) -> bool {
        self.booted
    }

    pub fn run_scripts<'a>(&self, sections: &[ScriptSection<'a>]) -> Result<()> {
        ensure_ruby()?;
        for section in sections {
            eval_section(section)?;
        }
        Ok(())
    }
}

fn ensure_ruby() -> Result<()> {
    RUBY_INIT
        .get_or_try_init(|| unsafe { start_ruby() })
        .map(|_| ())
}

unsafe fn start_ruby() -> Result<()> {
    let mut argc: c_int = 0;
    let mut argv: [*mut c_char; 0] = [];
    let mut argv_ptr = argv.as_mut_ptr();
    ruby_sysinit(&mut argc, &mut argv_ptr);

    let mut stack_marker: VALUE = 0;
    ruby_init_stack(addr_of_mut!(stack_marker) as *mut VALUE);

    let code = ruby_setup();
    if code != 0 {
        return Err(anyhow!("ruby_setup failed with code {code}"));
    }
    Ok(())
}

unsafe fn current_exception_message() -> String {
    let err = rb_errinfo();
    let mut string = rb_obj_as_string(err);
    let ptr = rb_string_value_cstr(&mut string);
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

fn eval_section(section: &ScriptSection<'_>) -> Result<()> {
    let label = script_label(section);
    debug!(target: "rgss", id = section.id, name = %label, "Evaluating script");
    let script = CString::new(section.source.as_bytes())
        .map_err(|_| anyhow!("script {label} contains interior null byte"))?;
    let mut state: c_int = 0;
    unsafe {
        rb_eval_string_protect(script.as_ptr(), &mut state);
        if state != 0 {
            let message = current_exception_message();
            return Err(anyhow!("Ruby error in script {label}: {message}"));
        }
    }
    Ok(())
}

fn script_label(section: &ScriptSection<'_>) -> String {
    let name = section.name.trim();
    if name.is_empty() {
        format!("Script {}", section.id)
    } else {
        name.to_string()
    }
}
