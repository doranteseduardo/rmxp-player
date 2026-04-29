//! Embedded Ruby (MRI) host for RGSS scripts.

mod audio;
mod classes;
mod fs;
mod graphics;
mod input;
mod kernel;
mod native;
mod runtime;
mod scripts;
mod system;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{
    rb_ary_new, rb_ary_push, rb_errinfo, rb_eval_string_protect, rb_gv_set, rb_intern,
    rb_obj_as_string, rb_require, rb_string_value_cstr, rb_utf8_str_new, ruby_init_loadpath,
    ruby_init_stack, ruby_options, ruby_setup, ruby_sysinit, VALUE,
};
use std::{
    env,
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    path::Path,
    ptr::addr_of_mut,
};
use tracing::{debug, info, warn};

static RUBY_INIT: OnceCell<()> = OnceCell::new();
const CURRENT_SCRIPT_GVAR: &[u8] = b"$RGSS_CURRENT_SCRIPT\0";
const FULL_MESSAGE_METHOD: &[u8] = b"full_message\0";

type ID = usize;

extern "C" {
    fn rb_funcall(recv: VALUE, mid: ID, argc: c_int, ...) -> VALUE;
}

pub use input::{
    update_input, InputSnapshot, TextEvent, BUTTON_A, BUTTON_ALT, BUTTON_B, BUTTON_C, BUTTON_CTRL,
    BUTTON_DOWN, BUTTON_F5, BUTTON_F6, BUTTON_F7, BUTTON_F8, BUTTON_F9, BUTTON_L, BUTTON_LEFT,
    BUTTON_MOUSE_LEFT, BUTTON_MOUSE_MIDDLE, BUTTON_MOUSE_RIGHT, BUTTON_MOUSE_X1, BUTTON_MOUSE_X2,
    BUTTON_R, BUTTON_RIGHT, BUTTON_SHIFT, BUTTON_UP, BUTTON_X, BUTTON_Y, BUTTON_Z,
};
pub use native::{
    bitmap_snapshot, drain_interpreter_commands, plane_snapshot, sprite_snapshot, tilemap_snapshot,
    viewport_snapshot, window_snapshot, BitmapData, InterpreterCommand, PlaneData, SpriteData,
    TilemapData, ViewportData, WindowData,
};
pub use system::{
    display_size, install_window_hooks, resize_window, set_game_title, set_platform_info,
    sync_window_dimensions, update_frame_delta, PlatformInfo, WindowHooks,
};

pub fn set_project_root(path: &Path) {
    native::set_project_root(path);
    fs::set_base_root(path);
}

pub fn set_config_dir(path: &Path) {
    native::set_config_dir(path);
}

pub fn set_save_dir(path: &Path) {
    native::set_save_dir(path);
}

pub fn sync_graphics_size(width: u32, height: u32) {
    graphics::set_screen_size(width, height);
}

pub use audio::{install_audio_hooks, AudioHooks, BgmCommand, BgsCommand, MeCommand, SeCommand};
pub use graphics::{
    current_frame_rate, request_hangup, screen_effects, store_backbuffer, tick as graphics_tick, ScreenEffects,
};
pub use runtime::{notify_low_memory, notify_resume, notify_suspend, MainResult};

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

/// Outcome of a single call to `resume_main_loop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainLoopOutcome {
    /// The main Fiber is still running; call again next frame.
    Active,
    /// The main Fiber finished cleanly; the game loop is done.
    Done,
    /// A `Reset` exception was raised; caller must re-evaluate scripts and
    /// call `resume_main_loop` again from the top.
    Reset,
}

pub struct RubyVm {
    booted: bool,
    main_active: bool,
}

pub struct ScriptSection<'a> {
    pub id: i32,
    pub name: &'a str,
    pub source: &'a str,
}

impl RubyVm {
    pub fn new() -> Self {
        Self {
            booted: false,
            main_active: false,
        }
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
            classes::init()?;
            audio::init()?;
            system::init()?;
            self.booted = true;
            scripts::load(self)?;
        }
        Ok(())
    }

    /// Evaluate a preload Ruby script with a given label (used in error messages).
    pub fn eval_preload(&self, code: &str, label: &str) -> Result<()> {
        let _guard = push_script_label(label);
        self.eval(code)
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

    pub fn run_scripts<'a>(&mut self, sections: &[ScriptSection<'a>]) -> Result<()> {
        ensure_ruby()?;
        // Reset the Fiber so a fresh main script fiber can be installed.
        let _ = runtime::reset_main();
        self.main_active = false;
        // Expose the script list to Ruby as $RGSS_SCRIPTS.
        unsafe { set_rgss_scripts_global(sections) };

        // Evaluate all sections except the last one (class/method definitions).
        // The last section is the Main script; instead of evaluating it
        // synchronously (which would block forever for games that run the game
        // loop in the top-level code), we wrap it in a Fiber driven by the
        // event loop via resume_main_loop().
        let (body, main_section) = match sections.split_last() {
            Some((last, rest)) => (rest, Some(last)),
            None => (sections, None),
        };
        for section in body {
            eval_section(section)?;
        }
        if let Some(main) = main_section {
            let label = script_label(main);
            let _guard = push_script_label(&label);
            if let Err(err) = runtime::install_main_from_source(main.source, &label) {
                warn!(target: "rgss", error = %err, "Failed to install main script fiber");
            }
        }

        self.main_active = runtime::is_main_active().unwrap_or(false);
        if !self.main_active {
            warn!(
                target: "rgss",
                "Scripts evaluated but main script fiber was not installed"
            );
        }
        Ok(())
    }

    pub fn resume_main_loop(&mut self) -> Result<MainLoopOutcome> {
        if !self.booted || !self.main_active {
            return Ok(MainLoopOutcome::Done);
        }
        match runtime::resume_main() {
            Ok(MainResult::Active) => Ok(MainLoopOutcome::Active),
            Ok(MainResult::Done) => {
                self.main_active = false;
                Ok(MainLoopOutcome::Done)
            }
            Ok(MainResult::Reset) => {
                self.main_active = false;
                Ok(MainLoopOutcome::Reset)
            }
            Err(err) => {
                self.main_active = false;
                Err(err)
            }
        }
    }

    pub fn has_main_loop(&self) -> bool {
        self.main_active
    }
}

/// Builds the $RGSS_SCRIPTS global: an Array of [id, name, ""] tuples.
/// Games inspect this to detect which scripts are loaded (e.g. Essentials version checks).
unsafe fn set_rgss_scripts_global(sections: &[ScriptSection<'_>]) {
    let outer = rb_ary_new();
    for section in sections {
        let inner = rb_ary_new();
        // id as Fixnum
        let id_val = (((section.id as i64) << rb_sys::ruby_special_consts::RUBY_SPECIAL_SHIFT as i64)
            | rb_sys::ruby_special_consts::RUBY_FIXNUM_FLAG as i64) as VALUE;
        rb_ary_push(inner, id_val);
        // name as UTF-8 String
        let name_bytes = section.name.as_bytes();
        let name_val = rb_utf8_str_new(name_bytes.as_ptr() as *const c_char, name_bytes.len() as i64);
        rb_ary_push(inner, name_val);
        // source placeholder — empty string (games only check index 1)
        let src_val = rb_utf8_str_new(b" ".as_ptr() as *const c_char, 0);
        rb_ary_push(inner, src_val);
        rb_ary_push(outer, inner);
    }
    const GVAR: &[u8] = b"$RGSS_SCRIPTS ";
    rb_gv_set(GVAR.as_ptr() as *const c_char, outer);
}

fn ensure_ruby() -> Result<()> {
    RUBY_INIT
        .get_or_try_init(|| unsafe { start_ruby() })
        .map(|_| ())
}

unsafe fn start_ruby() -> Result<()> {
    configure_ruby_load_path();
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
    ruby_init_loadpath();

    // Load Ruby's `<internal:*>` prelude files (kernel.rb, warning.rb, timev.rb,
    // etc.) which define core methods like Object#clone, Object#frozen?,
    // Kernel#warn, Integer#even?, Time.now. These are NOT loaded by
    // ruby_setup/ruby_init alone — only ruby_options triggers them as a side
    // effect of its parsing setup. We pass `-e ""` (empty script, never run)
    // because ruby_options requires a program. ruby_run_node is intentionally
    // not called: the empty script doesn't need to execute.
    let prog = CString::new("ruby")?;
    let dash_e = CString::new("-e")?;
    let empty = CString::new("")?;
    let mut prelude_argv: [*mut c_char; 3] = [
        prog.as_ptr() as *mut c_char,
        dash_e.as_ptr() as *mut c_char,
        empty.as_ptr() as *mut c_char,
    ];
    ruby_options(3, prelude_argv.as_mut_ptr());

    require_feature("enc/encdb")?;
    require_feature("enc/trans/transdb")?;
    Ok(())
}


fn configure_ruby_load_path() {
    let mut paths = Vec::new();
    add_cfg_path(option_env!("RGSS_RUBY_CFG_rubylibdir"), &mut paths);
    add_cfg_path(option_env!("RGSS_RUBY_CFG_archdir"), &mut paths);
    add_cfg_path(option_env!("RGSS_RUBY_CFG_sitearchdir"), &mut paths);
    add_cfg_path(option_env!("RGSS_RUBY_CFG_sitelibdir"), &mut paths);
    add_cfg_path(option_env!("RGSS_RUBY_CFG_vendorlibdir"), &mut paths);
    add_cfg_path(option_env!("RGSS_RUBY_CFG_vendorarchdir"), &mut paths);

    if let Some(existing) = env::var_os("RUBYLIB") {
        for path in env::split_paths(&existing) {
            if !paths.iter().any(|p| p == &path) {
                paths.push(path);
            }
        }
    }

    if paths.is_empty() {
        return;
    }

    match env::join_paths(&paths) {
        Ok(joined) => {
            debug!(target: "rgss", "Configured RUBYLIB with {} entries", paths.len());
            env::set_var("RUBYLIB", joined);
        }
        Err(err) => {
            warn!(
                target: "rgss",
                error = %err,
                "Failed to configure RUBYLIB"
            );
        }
    }
}

fn add_cfg_path(value: Option<&'static str>, paths: &mut Vec<std::path::PathBuf>) {
    if let Some(path) = value {
        if !path.is_empty() {
            let buf = std::path::PathBuf::from(path);
            if buf.exists() && !paths.iter().any(|p| p == &buf) {
                paths.push(buf);
            }
        }
    }
}

unsafe fn require_feature(feature: &str) -> Result<()> {
    let feature = CString::new(feature)?;
    rb_require(feature.as_ptr());
    Ok(())
}

pub(crate) unsafe fn current_exception_message() -> String {
    let err = rb_errinfo();
    let mut string = rb_obj_as_string(err);
    let ptr = rb_string_value_cstr(&mut string);
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

pub(crate) unsafe fn current_exception_full_message() -> Option<String> {
    let err = rb_errinfo();
    let mid = rb_intern(FULL_MESSAGE_METHOD.as_ptr() as *const c_char) as ID;
    let value = rb_funcall(err, mid, 0);
    if value == rb_sys::Qnil as VALUE {
        return None;
    }
    let mut string = value;
    let ptr = rb_string_value_cstr(&mut string);
    Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

fn eval_section(section: &ScriptSection<'_>) -> Result<()> {
    let label = script_label(section);
    debug!(target: "rgss", id = section.id, name = %label, "Evaluating script");
    let _guard = ScriptLabelGuard::push(&label);
    let script = CString::new(section.source.as_bytes())
        .map_err(|_| anyhow!("script {label} contains interior null byte"))?;
    let mut state: c_int = 0;
    unsafe {
        rb_eval_string_protect(script.as_ptr(), &mut state);
        if state != 0 {
            let message = current_exception_message();
            if let Some(full) = current_exception_full_message() {
                warn!(target: "rgss", script = %label, "{}", full);
            }
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

pub(crate) fn push_script_label(label: &str) -> ScriptLabelGuard {
    ScriptLabelGuard::push(label)
}

struct ScriptLabelGuard;

impl ScriptLabelGuard {
    fn push(label: &str) -> Self {
        unsafe { set_current_script_label(label) };
        Self
    }
}

impl Drop for ScriptLabelGuard {
    fn drop(&mut self) {
        unsafe {
            clear_current_script_label();
        }
    }
}

unsafe fn set_current_script_label(label: &str) {
    let bytes = label.as_bytes();
    let len = bytes.len() as i64;
    let value = rb_utf8_str_new(bytes.as_ptr() as *const c_char, len);
    rb_gv_set(CURRENT_SCRIPT_GVAR.as_ptr() as *const c_char, value);
}

unsafe fn clear_current_script_label() {
    rb_gv_set(
        CURRENT_SCRIPT_GVAR.as_ptr() as *const c_char,
        rb_sys::Qnil as VALUE,
    );
}
