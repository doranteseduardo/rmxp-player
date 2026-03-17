use super::{module, native_module, HandleStore};
use anyhow::{Context, Result};
use image::{ImageReader, RgbaImage};
use once_cell::sync::Lazy;
use rb_sys::{rb_num2int, rb_num2uint, rb_uint2inum, ruby_special_consts, special_consts, VALUE};
use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{info, warn};

const BITMAP_CREATE_NAME: &[u8] = b"bitmap_create\0";
const BITMAP_DISPOSE_NAME: &[u8] = b"bitmap_dispose\0";
const BITMAP_WIDTH_NAME: &[u8] = b"bitmap_width\0";
const BITMAP_HEIGHT_NAME: &[u8] = b"bitmap_height\0";
const BITMAP_DISPOSED_NAME: &[u8] = b"bitmap_disposed?\0";
const BITMAP_LOAD_NAME: &[u8] = b"bitmap_load\0";

static BITMAPS: Lazy<HandleStore<BitmapData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct BitmapData {
    pub width: u32,
    pub height: u32,
    pub disposed: bool,
    pub texture: Arc<RgbaImage>,
}

impl BitmapData {
    fn blank(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let texture = Arc::new(RgbaImage::from_pixel(
            width,
            height,
            image::Rgba([0, 0, 0, 0]),
        ));
        Self {
            width,
            height,
            disposed: false,
            texture,
        }
    }

    fn with_texture(texture: Arc<RgbaImage>) -> Self {
        let width = texture.width();
        let height = texture.height();
        Self {
            width,
            height,
            disposed: false,
            texture,
        }
    }
}

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE>,
        argc: c_int,
    );
    fn rb_string_value_cstr(ptr: *mut VALUE) -> *const c_char;
}

pub fn init() -> Result<()> {
    unsafe { define_bitmap_api() }
}

pub fn snapshot() -> Vec<(u32, BitmapData)> {
    BITMAPS.snapshot()
}

unsafe fn define_bitmap_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(BITMAP_CREATE_NAME), Some(bitmap_create), 2);
    rb_define_module_function(native, c_name(BITMAP_DISPOSE_NAME), Some(bitmap_dispose), 1);
    rb_define_module_function(native, c_name(BITMAP_WIDTH_NAME), Some(bitmap_width), 1);
    rb_define_module_function(native, c_name(BITMAP_HEIGHT_NAME), Some(bitmap_height), 1);
    rb_define_module_function(
        native,
        c_name(BITMAP_DISPOSED_NAME),
        Some(bitmap_disposed_q),
        1,
    );
    rb_define_module_function(native, c_name(BITMAP_LOAD_NAME), Some(bitmap_load), 1);
    Ok(())
}

unsafe extern "C" fn bitmap_create(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 1 || argv.is_null() {
        warn!(target: "rgss", "bitmap_create missing arguments");
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let width = clamp_dimension(rb_num2int(args[0]) as i32);
    let height = if args.len() >= 2 {
        clamp_dimension(rb_num2int(args[1]) as i32)
    } else {
        width
    };
    let id = store_bitmap(width, height);
    rb_uint2inum(id as usize)
}

unsafe extern "C" fn bitmap_load(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut arg = *argv;
    let path_ptr = rb_string_value_cstr(&mut arg);
    if path_ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let path = CStr::from_ptr(path_ptr).to_string_lossy().to_string();
    match load_bitmap_from_project(&path) {
        Ok(id) => rb_uint2inum(id as usize),
        Err(err) => {
            warn!(target: "rgss", path = %path, error = %err, "Failed to load bitmap");
            rb_sys::Qnil as VALUE
        }
    }
}

unsafe extern "C" fn bitmap_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    BITMAPS.with_mut(id, |entry| {
        entry.disposed = true;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_width(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let width = with_bitmap(id, |entry| entry.width).unwrap_or(0);
    int_to_value(width as i64)
}

unsafe extern "C" fn bitmap_height(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let height = with_bitmap(id, |entry| entry.height).unwrap_or(0);
    int_to_value(height as i64)
}

unsafe extern "C" fn bitmap_disposed_q(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qfalse as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let disposed = with_bitmap(id, |entry| entry.disposed).unwrap_or(true);
    if disposed {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

fn store_bitmap(width: u32, height: u32) -> u32 {
    store_bitmap_data(BitmapData::blank(width, height))
}

fn with_bitmap<F, R>(id: u32, func: F) -> Option<R>
where
    F: FnOnce(&BitmapData) -> R,
{
    BITMAPS.with(id, func)
}

fn clamp_dimension(value: i32) -> u32 {
    value.max(1) as u32
}

fn int_to_value(value: i64) -> VALUE {
    unsafe {
        if value >= special_consts::FIXNUM_MIN as i64 && value <= special_consts::FIXNUM_MAX as i64
        {
            ((value << ruby_special_consts::RUBY_SPECIAL_SHIFT as i64)
                | ruby_special_consts::RUBY_FIXNUM_FLAG as i64) as VALUE
        } else {
            rb_sys::rb_int2big(value as isize)
        }
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}

fn store_bitmap_data(data: BitmapData) -> u32 {
    BITMAPS.insert(data)
}

fn load_bitmap_from_project(relative: &str) -> Result<u32> {
    let root = module::project_root().ok_or_else(|| anyhow::anyhow!("project root not set"))?;
    let candidates = candidate_paths(relative);
    for candidate in candidates {
        let path = if candidate.is_absolute() {
            candidate
        } else {
            root.join(&candidate)
        };
        if path.exists() {
            return load_bitmap_from_file(&path);
        }
    }
    anyhow::bail!("{} not found under {}", relative, root.display())
}

fn candidate_paths(relative: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let base = PathBuf::from(relative);
    paths.push(base.clone());
    if base.extension().is_none() {
        paths.push(base.with_extension("png"));
        paths.push(base.with_extension("PNG"));
    }
    paths
}

fn load_bitmap_from_file(path: &Path) -> Result<u32> {
    let image = ImageReader::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .decode()
        .with_context(|| format!("decoding {}", path.display()))?
        .to_rgba8();
    info!(
        target: "rgss",
        file = %path.display(),
        width = image.width(),
        height = image.height(),
        "bitmap loaded"
    );
    Ok(store_bitmap_data(BitmapData::with_texture(Arc::new(image))))
}
