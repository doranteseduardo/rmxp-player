use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{rb_define_module, rb_define_module_under, rb_utf8_str_new, VALUE};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
};

const RGSS_MODULE_NAME: &[u8] = b"RGSS\0";
const NATIVE_MODULE_NAME: &[u8] = b"Native\0";
const PROJECT_PATH_NAME: &[u8] = b"project_path\0";

static RGSS_MODULE: OnceCell<VALUE> = OnceCell::new();
static NATIVE_MODULE: OnceCell<VALUE> = OnceCell::new();
static PROJECT_ROOT: OnceCell<PathBuf> = OnceCell::new();
static NATIVE_FUNCTIONS: OnceCell<()> = OnceCell::new();

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE>,
        argc: c_int,
    );
    fn rb_string_value_cstr(value: *mut VALUE) -> *const c_char;
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

pub fn project_root() -> Option<&'static PathBuf> {
    PROJECT_ROOT.get()
}

pub fn resolve_project_path(relative: &str) -> Option<PathBuf> {
    if relative.is_empty() {
        return project_root().cloned();
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        project_root().map(|root| root.join(path))
    }
}

unsafe fn define_native_functions(module: VALUE) -> Result<()> {
    rb_define_module_function(
        module,
        c_name(PROJECT_PATH_NAME),
        Some(native_project_path),
        -1,
    );
    Ok(())
}

unsafe extern "C" fn native_project_path(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        if let Some(root) = project_root() {
            return path_to_value(root);
        }
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let mut value = args[0];
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let text = CStr::from_ptr(ptr).to_string_lossy();
    match resolve_project_path(text.trim()) {
        Some(path) => path_to_value(&path),
        None => rb_sys::Qnil as VALUE,
    }
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
