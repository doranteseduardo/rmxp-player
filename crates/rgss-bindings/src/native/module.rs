use crate::fs;
use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{rb_define_module, rb_define_module_under, rb_obj_class, rb_utf8_str_new, VALUE};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int, c_long},
    path::{Path, PathBuf},
};

const RGSS_MODULE_NAME: &[u8] = b"RGSS\0";
const NATIVE_MODULE_NAME: &[u8] = b"Native\0";
const PROJECT_PATH_NAME: &[u8] = b"project_path\0";
const CONFIG_PATH_NAME: &[u8] = b"config_path\0";
const SAVE_PATH_NAME: &[u8] = b"save_path\0";
const CLASS_OF_NAME: &[u8] = b"class_of\0";
const MARSHAL_LOAD_NAME: &[u8] = b"marshal_load\0";

static RGSS_MODULE: OnceCell<VALUE> = OnceCell::new();
static NATIVE_MODULE: OnceCell<VALUE> = OnceCell::new();
static PROJECT_ROOT: OnceCell<PathBuf> = OnceCell::new();
static CONFIG_DIR: OnceCell<PathBuf> = OnceCell::new();
static SAVE_DIR: OnceCell<PathBuf> = OnceCell::new();
static NATIVE_FUNCTIONS: OnceCell<()> = OnceCell::new();

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE>,
        argc: c_int,
    );
    fn rb_string_value_cstr(value: *mut VALUE) -> *const c_char;
    fn rb_marshal_load(port: VALUE) -> VALUE;
    fn rb_str_new(ptr: *const c_char, len: c_long) -> VALUE;
}

pub fn init() -> Result<()> {
    rgss_module()?;
    let native = native_module()?;
    NATIVE_FUNCTIONS
        .get_or_try_init(|| unsafe { define_native_functions(native) })
        .map(|_| ())
}

pub fn rgss_module() -> Result<VALUE> {
    RGSS_MODULE
        .get_or_try_init(|| unsafe {
            let module = rb_define_module(c_name(RGSS_MODULE_NAME));
            if module == 0 {
                Err(anyhow!("failed to define RGSS module"))
            } else {
                Ok(module)
            }
        })
        .copied()
}

pub fn native_module() -> Result<VALUE> {
    NATIVE_MODULE
        .get_or_try_init(|| unsafe {
            let rgss = rgss_module()?;
            let module = rb_define_module_under(rgss, c_name(NATIVE_MODULE_NAME));
            if module == 0 {
                Err(anyhow!("failed to define RGSS::Native module"))
            } else {
                Ok(module)
            }
        })
        .copied()
}

pub fn set_project_root(path: impl AsRef<Path>) {
    let _ = PROJECT_ROOT.set(path.as_ref().to_path_buf());
}

pub fn set_config_dir(path: impl AsRef<Path>) {
    let _ = CONFIG_DIR.set(path.as_ref().to_path_buf());
}

pub fn set_save_dir(path: impl AsRef<Path>) {
    let _ = SAVE_DIR.set(path.as_ref().to_path_buf());
}

pub fn project_root() -> Option<&'static PathBuf> {
    PROJECT_ROOT.get()
}

pub fn config_dir() -> Option<&'static PathBuf> {
    CONFIG_DIR.get()
}

pub fn save_dir() -> Option<&'static PathBuf> {
    SAVE_DIR.get()
}

pub fn resolve_project_path(relative: &str) -> Option<PathBuf> {
    if relative.trim().is_empty() {
        return project_root().cloned();
    }
    fs::resolve(relative)
}

unsafe fn define_native_functions(module: VALUE) -> Result<()> {
    rb_define_module_function(
        module,
        c_name(PROJECT_PATH_NAME),
        Some(native_project_path),
        -1,
    );
    rb_define_module_function(
        module,
        c_name(CONFIG_PATH_NAME),
        Some(native_config_path),
        0,
    );
    rb_define_module_function(module, c_name(SAVE_PATH_NAME), Some(native_save_path), -1);
    rb_define_module_function(module, c_name(CLASS_OF_NAME), Some(native_class_of), -1);
    rb_define_module_function(module, c_name(MARSHAL_LOAD_NAME), Some(native_marshal_load), -1);
    Ok(())
}

unsafe extern "C" fn native_project_path(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        return match fs::data_root().or_else(|| project_root().cloned()) {
            Some(root) => path_to_value(&root),
            None => rb_sys::Qnil as VALUE,
        };
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut value = args[0];
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let text = CStr::from_ptr(ptr).to_string_lossy();
    match fs::resolve(text.trim()).or_else(|| resolve_project_path(text.trim())) {
        Some(path) => path_to_value(&path),
        None => rb_sys::Qnil as VALUE,
    }
}

unsafe extern "C" fn native_config_path(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    match config_dir() {
        Some(path) => path_to_value(path),
        None => rb_sys::Qnil as VALUE,
    }
}

unsafe extern "C" fn native_save_path(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    match save_dir() {
        Some(path) => path_to_value(path),
        None => rb_sys::Qnil as VALUE,
    }
}

unsafe extern "C" fn native_class_of(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    rb_obj_class(args[0])
}

/// RGSS::Native.marshal_load(path_string) -> Object
/// Reads the file at `path_string` in Rust and deserializes it with the C-level
/// rb_marshal_load, bypassing any Ruby-level Marshal/IO method dispatch corruption.
unsafe extern "C" fn native_marshal_load(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut val = args[0];
    let ptr = rb_string_value_cstr(&mut val);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let path = CStr::from_ptr(ptr).to_string_lossy();
    let bytes = match std::fs::read(path.as_ref()) {
        Ok(b) => b,
        Err(_) => return rb_sys::Qnil as VALUE,
    };
    let rb_str = rb_str_new(bytes.as_ptr() as *const c_char, bytes.len() as c_long);
    rb_marshal_load(rb_str)
}

fn path_to_value(path: &Path) -> VALUE {
    if let Some(text) = path.to_str() {
        if let Ok(c_string) = CString::new(text) {
            let len = c_string.as_bytes().len() as i64;
            unsafe { rb_utf8_str_new(c_string.as_ptr(), len) }
        } else {
            rb_sys::Qnil as VALUE
        }
    } else {
        rb_sys::Qnil as VALUE
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
