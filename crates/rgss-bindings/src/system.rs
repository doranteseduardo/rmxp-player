use crate::native::{config_dir, project_root, save_dir};
use anyhow::{anyhow, Result};
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    rb_define_const, rb_define_module, rb_float_new, rb_hash_aset, rb_hash_new, rb_id2sym,
    rb_intern, rb_string_value_cstr, rb_utf8_str_new, VALUE,
};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    sync::RwLock,
    time::{Duration, Instant},
};
use tracing::warn;

const SYSTEM_MODULE_NAME: &[u8] = b"System\0";
const DELTA_NAME: &[u8] = b"delta\0";
const UPTIME_NAME: &[u8] = b"uptime\0";
const SET_WINDOW_TITLE_NAME: &[u8] = b"set_window_title\0";
const WINDOW_TITLE_NAME: &[u8] = b"window_title\0";
const WINDOW_TITLE_SET_NAME: &[u8] = b"window_title=\0";
const GAME_TITLE_NAME: &[u8] = b"game_title\0";
const USER_LANGUAGE_NAME: &[u8] = b"user_language\0";
const USER_NAME_NAME: &[u8] = b"user_name\0";
const POWER_STATE_NAME: &[u8] = b"power_state\0";
const RELOAD_CACHE_NAME: &[u8] = b"reload_cache\0";
const MOUNT_NAME: &[u8] = b"mount\0";
const DATA_DIRECTORY_NAME: &[u8] = b"data_directory\0";
const CONFIG_DIRECTORY_NAME: &[u8] = b"config_directory\0";
const SAVE_DIRECTORY_NAME: &[u8] = b"save_directory\0";
const PUTS_NAME: &[u8] = b"puts\0";
const VERSION_NAME: &[u8] = b"VERSION\0";
const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

static SYSTEM_MODULE: OnceCell<VALUE> = OnceCell::new();
static START_TIME: OnceCell<Instant> = OnceCell::new();
static WINDOW_HOOKS: OnceCell<WindowHooks> = OnceCell::new();
static PLATFORM_INFO: Lazy<RwLock<PlatformInfo>> = Lazy::new(|| {
    RwLock::new(PlatformInfo {
        platform: "Unknown".into(),
        user_name: "Player".into(),
        user_language: "en_US".into(),
    })
});
static GAME_TITLE: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new("RMXP Native Player".into()));
static FRAME_DELTA_SECS: OnceCell<RwLock<f64>> = OnceCell::new();
static WINDOW_DIMENSIONS: Lazy<RwLock<(u32, u32)>> = Lazy::new(|| RwLock::new((640, 480)));

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
}

#[derive(Clone)]
pub struct PlatformInfo {
    pub platform: String,
    pub user_name: String,
    pub user_language: String,
}

pub struct WindowHooks {
    pub set_title: fn(&str),
    pub get_title: fn() -> String,
    pub set_inner_size: fn(u32, u32),
    pub get_display_size: fn() -> (u32, u32),
    pub center: fn(),
    pub set_fullscreen: fn(bool),
    pub set_cursor_visible: fn(bool),
}

pub fn init() -> Result<()> {
    SYSTEM_MODULE
        .get_or_try_init(|| unsafe {
            START_TIME.get_or_init(Instant::now);
            let module = rb_define_module(c_name(SYSTEM_MODULE_NAME));
            if module == 0 {
                Err(anyhow!("failed to define System module"))
            } else {
                define_system_functions(module)?;
                rb_define_const(
                    module,
                    c_name(VERSION_NAME),
                    string_to_value(ENGINE_VERSION),
                );
                Ok(module)
            }
        })
        .map(|_| ())
}

pub fn install_window_hooks(hooks: WindowHooks) {
    let _ = WINDOW_HOOKS.set(hooks);
}

pub fn set_platform_info(info: PlatformInfo) {
    if let Ok(mut guard) = PLATFORM_INFO.write() {
        *guard = info;
    }
}

pub fn set_game_title(title: impl Into<String>) {
    if let Ok(mut guard) = GAME_TITLE.write() {
        *guard = title.into();
    }
}

pub fn update_frame_delta(delta: f64) {
    let lock = FRAME_DELTA_SECS.get_or_init(|| RwLock::new(1.0 / 60.0));
    if let Ok(mut guard) = lock.write() {
        *guard = delta;
    }
}

pub fn current_delta() -> f64 {
    let lock = FRAME_DELTA_SECS.get_or_init(|| RwLock::new(1.0 / 60.0));
    lock.read().map(|v| *v).unwrap_or(1.0 / 60.0)
}

pub fn resize_window(width: u32, height: u32) {
    set_window_dimensions(width, height);
    if let Some(hooks) = WINDOW_HOOKS.get() {
        (hooks.set_inner_size)(width.max(1), height.max(1));
    } else {
        warn!(target: "system", "System.resize_window called before hooks installed");
    }
}

pub fn display_size() -> (u32, u32) {
    if let Some(hooks) = WINDOW_HOOKS.get() {
        return (hooks.get_display_size)();
    }
    WINDOW_DIMENSIONS
        .read()
        .map(|dims| *dims)
        .unwrap_or((640, 480))
}

pub fn center_window() {
    if let Some(hooks) = WINDOW_HOOKS.get() {
        (hooks.center)();
    } else {
        warn!(target: "system", "System.center_window called before hooks installed");
    }
}

pub fn set_fullscreen(enable: bool) {
    if let Some(hooks) = WINDOW_HOOKS.get() {
        (hooks.set_fullscreen)(enable);
    } else {
        warn!(
            target: "system",
            "System.set_fullscreen called before hooks installed"
        );
    }
}

pub fn set_cursor_visible(show: bool) {
    if let Some(hooks) = WINDOW_HOOKS.get() {
        (hooks.set_cursor_visible)(show);
    } else {
        warn!(
            target: "system",
            "System.set_cursor_visible called before hooks installed"
        );
    }
}

pub fn sync_window_dimensions(width: u32, height: u32) {
    set_window_dimensions(width, height);
}

fn set_window_dimensions(width: u32, height: u32) {
    if let Ok(mut guard) = WINDOW_DIMENSIONS.write() {
        *guard = (width.max(1), height.max(1));
    }
}

unsafe fn define_system_functions(module: VALUE) -> Result<()> {
    rb_define_module_function(module, c_name(DELTA_NAME), Some(system_delta), 0);
    rb_define_module_function(module, c_name(UPTIME_NAME), Some(system_uptime), 0);
    rb_define_module_function(
        module,
        c_name(SET_WINDOW_TITLE_NAME),
        Some(system_set_window_title),
        1,
    );
    rb_define_module_function(
        module,
        c_name(WINDOW_TITLE_NAME),
        Some(system_window_title),
        0,
    );
    rb_define_module_function(
        module,
        c_name(WINDOW_TITLE_SET_NAME),
        Some(system_set_window_title),
        1,
    );
    rb_define_module_function(module, c_name(GAME_TITLE_NAME), Some(system_game_title), 0);
    rb_define_module_function(
        module,
        c_name(USER_LANGUAGE_NAME),
        Some(system_user_language),
        0,
    );
    rb_define_module_function(module, c_name(USER_NAME_NAME), Some(system_user_name), 0);
    rb_define_module_function(
        module,
        c_name(POWER_STATE_NAME),
        Some(system_power_state),
        0,
    );
    rb_define_module_function(
        module,
        c_name(RELOAD_CACHE_NAME),
        Some(system_reload_cache),
        0,
    );
    rb_define_module_function(module, c_name(MOUNT_NAME), Some(system_mount), -1);
    rb_define_module_function(
        module,
        c_name(DATA_DIRECTORY_NAME),
        Some(system_data_directory),
        0,
    );
    rb_define_module_function(
        module,
        c_name(CONFIG_DIRECTORY_NAME),
        Some(system_config_directory),
        0,
    );
    rb_define_module_function(
        module,
        c_name(SAVE_DIRECTORY_NAME),
        Some(system_save_directory),
        0,
    );
    rb_define_module_function(module, c_name(PUTS_NAME), Some(system_puts), -1);
    Ok(())
}

unsafe extern "C" fn system_delta(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    rb_float_new(current_delta())
}

unsafe extern "C" fn system_uptime(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let now = Instant::now();
    let start = *START_TIME.get_or_init(Instant::now);
    let elapsed = now.saturating_duration_since(start);
    rb_float_new(duration_to_secs(elapsed))
}

unsafe extern "C" fn system_set_window_title(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc <= 0 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    if let Some(hooks) = WINDOW_HOOKS.get() {
        let args = std::slice::from_raw_parts(argv, argc as usize);
        let mut value = args[0];
        let ptr = rb_string_value_cstr(&mut value);
        if !ptr.is_null() {
            let text = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
            (hooks.set_title)(text.as_str());
            set_game_title(text);
        }
    } else {
        warn!(target: "system", "System.set_window_title called before hooks installed");
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_window_title(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    match WINDOW_HOOKS.get() {
        Some(hooks) => string_to_value(&(hooks.get_title)()),
        None => string_to_value(&GAME_TITLE.read().map(|s| s.clone()).unwrap_or_default()),
    }
}

unsafe extern "C" fn system_game_title(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    string_to_value(&GAME_TITLE.read().map(|s| s.clone()).unwrap_or_default())
}

unsafe extern "C" fn system_user_language(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let info = PLATFORM_INFO.read().ok();
    string_to_value(&info.map(|i| i.user_language.clone()).unwrap_or_default())
}

unsafe extern "C" fn system_user_name(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let info = PLATFORM_INFO.read().ok();
    string_to_value(&info.map(|i| i.user_name.clone()).unwrap_or_default())
}

unsafe extern "C" fn system_power_state(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let hash = rb_hash_new();
    rb_hash_aset(hash, symbol("discharging"), bool_to_value(false));
    rb_hash_aset(hash, symbol("seconds"), rb_sys::Qnil as VALUE);
    rb_hash_aset(hash, symbol("percent"), rb_sys::Qnil as VALUE);
    hash
}

unsafe extern "C" fn system_reload_cache(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    warn!(target: "system", "System.reload_cache not implemented yet");
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_mount(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc == 0 || argv.is_null() {
        warn!(target: "system", "System.mount requires at least one argument");
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut value = args[0];
    let ptr = rb_string_value_cstr(&mut value);
    if !ptr.is_null() {
        let text = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
        warn!(target: "system", path = %text, "System.mount is a no-op in the native player");
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_data_directory(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if let Some(root) = project_root() {
        return path_to_value(root);
    }
    match std::env::current_dir() {
        Ok(path) => path_to_value(&path),
        Err(_) => rb_sys::Qnil as VALUE,
    }
}

unsafe extern "C" fn system_config_directory(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if let Some(dir) = config_dir() {
        return path_to_value(dir);
    }
    system_data_directory(0, std::ptr::null(), _self)
}

unsafe extern "C" fn system_save_directory(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if let Some(dir) = save_dir() {
        return path_to_value(dir);
    }
    system_config_directory(0, std::ptr::null(), _self)
}

unsafe extern "C" fn system_puts(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        println!();
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    for value in args {
        let mut current = *value;
        let ptr = rb_string_value_cstr(&mut current);
        if ptr.is_null() {
            println!();
        } else {
            let text = CStr::from_ptr(ptr).to_string_lossy();
            println!("{text}");
        }
    }
    rb_sys::Qnil as VALUE
}

fn duration_to_secs(duration: Duration) -> f64 {
    duration.as_secs_f64()
}

fn string_to_value(text: &str) -> VALUE {
    unsafe {
        match CString::new(text) {
            Ok(cstr) => rb_utf8_str_new(cstr.as_ptr(), text.len() as i64),
            Err(_) => rb_sys::Qnil as VALUE,
        }
    }
}

fn path_to_value(path: &std::path::Path) -> VALUE {
    match path.to_str() {
        Some(text) => string_to_value(text),
        None => rb_sys::Qnil as VALUE,
    }
}

fn symbol(name: &str) -> VALUE {
    unsafe {
        let cstr = CString::new(name).expect("symbol name");
        rb_id2sym(rb_intern(cstr.as_ptr()))
    }
}

fn bool_to_value(value: bool) -> VALUE {
    if value {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
