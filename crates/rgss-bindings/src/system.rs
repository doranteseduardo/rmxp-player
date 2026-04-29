use crate::{
    fs,
    native::{config_dir, save_dir, value_to_bool},
};
use anyhow::{anyhow, Result};
use csv::ReaderBuilder;
use once_cell::sync::{Lazy, OnceCell};
use open::that_detached;
use rb_sys::{
    rb_ary_new, rb_ary_push, rb_define_const, rb_define_module, rb_float_new, rb_hash_aset,
    rb_hash_new, rb_id2sym, rb_intern, rb_ll2inum, rb_string_value_cstr, rb_utf8_str_new, VALUE,
};
use std::{
    env,
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::RwLock,
    thread,
    time::{Duration, Instant},
};
use sysinfo::System;
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
const UNMOUNT_NAME: &[u8] = b"unmount\0";
const DATA_DIRECTORY_NAME: &[u8] = b"data_directory\0";
const CONFIG_DIRECTORY_NAME: &[u8] = b"config_directory\0";
const SAVE_DIRECTORY_NAME: &[u8] = b"save_directory\0";
const PUTS_NAME: &[u8] = b"puts\0";
const VERSION_NAME: &[u8] = b"VERSION\0";
const SHOW_SETTINGS_NAME: &[u8] = b"show_settings\0";
const DESENSITIZE_NAME: &[u8] = b"desensitize\0";
const PLATFORM_NAME: &[u8] = b"platform\0";
const IS_MAC_Q_NAME: &[u8] = b"is_mac?\0";
const IS_LINUX_Q_NAME: &[u8] = b"is_linux?\0";
const IS_WINDOWS_Q_NAME: &[u8] = b"is_windows?\0";
const IS_REAL_MAC_Q_NAME: &[u8] = b"is_really_mac?\0";
const IS_REAL_LINUX_Q_NAME: &[u8] = b"is_really_linux?\0";
const IS_REAL_WINDOWS_Q_NAME: &[u8] = b"is_really_windows?\0";
const IS_ROSETTA_Q_NAME: &[u8] = b"is_rosetta?\0";
const IS_WINE_Q_NAME: &[u8] = b"is_wine?\0";
const NPROC_NAME: &[u8] = b"nproc\0";
const MEMORY_NAME: &[u8] = b"memory\0";
const FILE_EXIST_Q_NAME: &[u8] = b"file_exist?\0";
const LAUNCH_NAME: &[u8] = b"launch\0";
const DEFAULT_FONT_FAMILY_SET_NAME: &[u8] = b"default_font_family=\0";
const PARSE_CSV_NAME: &[u8] = b"parse_csv\0";
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
static WINDOW_DIMENSIONS: Lazy<RwLock<(u32, u32)>> = Lazy::new(|| RwLock::new((512, 384)));
static DEFAULT_FONT_FAMILY: Lazy<RwLock<Option<String>>> = Lazy::new(|| RwLock::new(None));

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
    rb_define_module_function(module, c_name(DELTA_NAME), Some(system_delta), -1);
    rb_define_module_function(module, c_name(UPTIME_NAME), Some(system_uptime), -1);
    rb_define_module_function(
        module,
        c_name(SET_WINDOW_TITLE_NAME),
        Some(system_set_window_title),
        -1,
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
        -1,
    );
    rb_define_module_function(module, c_name(GAME_TITLE_NAME), Some(system_game_title), -1);
    rb_define_module_function(
        module,
        c_name(USER_LANGUAGE_NAME),
        Some(system_user_language),
        0,
    );
    rb_define_module_function(module, c_name(USER_NAME_NAME), Some(system_user_name), -1);
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
    rb_define_module_function(
        module,
        c_name(SHOW_SETTINGS_NAME),
        Some(system_show_settings),
        0,
    );
    rb_define_module_function(
        module,
        c_name(DESENSITIZE_NAME),
        Some(system_desensitize),
        -1,
    );
    rb_define_module_function(module, c_name(PLATFORM_NAME), Some(system_platform), -1);
    rb_define_module_function(module, c_name(IS_MAC_Q_NAME), Some(system_is_mac_q), -1);
    rb_define_module_function(module, c_name(IS_LINUX_Q_NAME), Some(system_is_linux_q), -1);
    rb_define_module_function(
        module,
        c_name(IS_WINDOWS_Q_NAME),
        Some(system_is_windows_q),
        0,
    );
    rb_define_module_function(module, c_name(IS_REAL_MAC_Q_NAME), Some(system_is_mac_q), -1);
    rb_define_module_function(
        module,
        c_name(IS_REAL_LINUX_Q_NAME),
        Some(system_is_linux_q),
        0,
    );
    rb_define_module_function(
        module,
        c_name(IS_REAL_WINDOWS_Q_NAME),
        Some(system_is_windows_q),
        0,
    );
    rb_define_module_function(
        module,
        c_name(IS_ROSETTA_Q_NAME),
        Some(system_is_rosetta_q),
        0,
    );
    rb_define_module_function(module, c_name(IS_WINE_Q_NAME), Some(system_is_wine_q), -1);
    rb_define_module_function(module, c_name(NPROC_NAME), Some(system_nproc), -1);
    rb_define_module_function(module, c_name(MEMORY_NAME), Some(system_memory), -1);
    rb_define_module_function(
        module,
        c_name(FILE_EXIST_Q_NAME),
        Some(system_file_exist_q),
        -1,
    );
    rb_define_module_function(module, c_name(LAUNCH_NAME), Some(system_launch), -1);
    rb_define_module_function(
        module,
        c_name(DEFAULT_FONT_FAMILY_SET_NAME),
        Some(system_set_default_font_family),
        -1,
    );
    rb_define_module_function(module, c_name(PARSE_CSV_NAME), Some(system_parse_csv), -1);
    rb_define_module_function(module, c_name(UNMOUNT_NAME), Some(system_unmount), -1);
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
    fs::reload();
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_mount(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc == 0 || argv.is_null() {
        warn!(target: "system", "System.mount requires at least one argument");
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut path_value = args[0];
    let ptr = rb_string_value_cstr(&mut path_value);
    if ptr.is_null() {
        warn!(target: "system", "System.mount path must be a String");
        return rb_sys::Qnil as VALUE;
    }
    let path = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    let mountpoint = if args.len() >= 2 {
        let mut mount_value = args[1];
        if mount_value == rb_sys::Qnil as VALUE {
            None
        } else {
            let ptr = rb_string_value_cstr(&mut mount_value);
            if ptr.is_null() {
                None
            } else {
                Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
            }
        }
    } else {
        None
    };
    let reload = if args.len() >= 3 {
        value_to_bool(args[2])
    } else {
        true
    };
    let Some(resolved) = fs::resolve_mount_source(&path) else {
        warn!(target: "system", path = %path, "Mount source could not be resolved");
        return rb_sys::Qnil as VALUE;
    };
    let mountpoint_path = mountpoint
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(fs::clean_mountpoint);
    if fs::mount_path(resolved, mountpoint_path) && reload {
        fs::reload();
    }
    args[0]
}

unsafe extern "C" fn system_unmount(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc == 0 || argv.is_null() {
        warn!(target: "system", "System.unmount requires at least one argument");
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut path_value = args[0];
    let ptr = rb_string_value_cstr(&mut path_value);
    if ptr.is_null() {
        warn!(target: "system", "System.unmount path must be a String");
        return rb_sys::Qnil as VALUE;
    }
    let path = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    let reload = if args.len() >= 2 {
        value_to_bool(args[1])
    } else {
        true
    };
    let Some(resolved) = fs::resolve_mount_source(&path) else {
        warn!(target: "system", path = %path, "Unmount source could not be resolved");
        return rb_sys::Qnil as VALUE;
    };
    if fs::unmount_path(&resolved, None) && reload {
        fs::reload();
    }
    args[0]
}

unsafe extern "C" fn system_data_directory(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if let Some(root) = fs::data_root() {
        return path_to_value(&root);
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

unsafe extern "C" fn system_show_settings(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    warn!(target: "system", "System.show_settings is not implemented; ignoring");
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_desensitize(_argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut value = *argv;
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    let resolved = fs::desensitize(&text).unwrap_or_else(|| PathBuf::from(text));
    string_to_value(&path_to_string(&resolved))
}

unsafe extern "C" fn system_platform(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let platform = PLATFORM_INFO
        .read()
        .ok()
        .map(|info| info.platform.clone())
        .unwrap_or_else(detect_platform_string);
    string_to_value(&platform)
}

unsafe extern "C" fn system_is_mac_q(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(cfg!(target_os = "macos"))
}

unsafe extern "C" fn system_is_linux_q(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(cfg!(target_os = "linux"))
}

unsafe extern "C" fn system_is_windows_q(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(cfg!(target_os = "windows"))
}

unsafe extern "C" fn system_is_rosetta_q(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(is_rosetta())
}

unsafe extern "C" fn system_is_wine_q(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(is_wine())
}

unsafe extern "C" fn system_nproc(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let count = thread::available_parallelism()
        .map(|n| n.get() as i64)
        .unwrap_or(1);
    int_to_value(count)
}

unsafe extern "C" fn system_memory(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    match total_memory_mb() {
        Some(mb) => int_to_value(mb),
        None => rb_sys::Qnil as VALUE,
    }
}

unsafe extern "C" fn system_file_exist_q(_argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qfalse as VALUE;
    }
    let mut value = *argv;
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qfalse as VALUE;
    }
    let text = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    let exists = fs::exists(&text);
    bool_to_value(exists)
}

unsafe extern "C" fn system_launch(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc == 0 || argv.is_null() {
        warn!(target: "system", "System.launch requires at least one argument");
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut target_value = args[0];
    let ptr = rb_string_value_cstr(&mut target_value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let target_path = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    if let Err(err) = that_detached(&target_path) {
        warn!(target: "system", error = %err, path = %target_path, "System.launch failed");
    }
    if argc > 1 {
        warn!(
            target: "system",
            "System.launch ignores additional arguments on this platform"
        );
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_set_default_font_family(
    _argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut value = *argv;
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let family = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    if let Ok(mut guard) = DEFAULT_FONT_FAMILY.write() {
        *guard = Some(family);
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn system_parse_csv(_argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut value = *argv;
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let data = CStr::from_ptr(ptr).to_string_lossy().into_owned();
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .double_quote(true)
        .from_reader(data.as_bytes());
    let rows = rb_ary_new();
    for record in reader.records() {
        match record {
            Ok(fields) => {
                let row = rb_ary_new();
                for field in fields.iter() {
                    let cell = rb_utf8_str_new(field.as_ptr() as *const c_char, field.len() as i64);
                    rb_ary_push(row, cell);
                }
                rb_ary_push(rows, row);
            }
            Err(err) => {
                warn!(target: "system", error = %err, "System.parse_csv failed");
                return rb_sys::Qnil as VALUE;
            }
        }
    }
    rows
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

fn detect_platform_string() -> String {
    if cfg!(target_os = "macos") {
        "macOS".into()
    } else if cfg!(target_os = "windows") {
        "Windows".into()
    } else if cfg!(target_os = "linux") {
        "Linux".into()
    } else {
        env::consts::OS.into()
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn total_memory_mb() -> Option<i64> {
    let mut sys = System::new();
    sys.refresh_memory();
    let kb = sys.total_memory();
    Some((kb as i64 / 1024).max(0))
}

fn is_wine() -> bool {
    env::var("WINEPREFIX").is_ok() || env::var("WINELOADERNOEXEC").is_ok()
}

#[cfg(target_os = "macos")]
fn is_rosetta() -> bool {
    env::var("RMXP_NATIVE_ROSETTA")
        .map(|value| value == "1")
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_rosetta() -> bool {
    false
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

fn int_to_value(value: i64) -> VALUE {
    unsafe { rb_ll2inum(value) }
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
