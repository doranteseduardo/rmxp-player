use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{rb_define_module, rb_define_module_under, VALUE};
use std::{
    os::raw::c_char,
    path::{Path, PathBuf},
};

const RGSS_MODULE_NAME: &[u8] = b"RGSS\0";
const NATIVE_MODULE_NAME: &[u8] = b"Native\0";

static RGSS_MODULE: OnceCell<VALUE> = OnceCell::new();
static NATIVE_MODULE: OnceCell<VALUE> = OnceCell::new();
static PROJECT_ROOT: OnceCell<PathBuf> = OnceCell::new();

pub fn init() -> Result<()> {
    rgss_module()?;
    native_module()?;
    Ok(())
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

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
