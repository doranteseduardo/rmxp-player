use super::{native_module, HandleStore};
use anyhow::Result;
use once_cell::sync::Lazy;
use rb_sys::{rb_num2int, rb_num2uint, rb_uint2inum, ruby_special_consts, special_consts, VALUE};
use std::os::raw::{c_char, c_int};
use tracing::warn;

const BITMAP_CREATE_NAME: &[u8] = b"bitmap_create\0";
const BITMAP_DISPOSE_NAME: &[u8] = b"bitmap_dispose\0";
const BITMAP_WIDTH_NAME: &[u8] = b"bitmap_width\0";
const BITMAP_HEIGHT_NAME: &[u8] = b"bitmap_height\0";
const BITMAP_DISPOSED_NAME: &[u8] = b"bitmap_disposed?\0";

static BITMAPS: Lazy<HandleStore<BitmapData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct BitmapData {
    pub width: u32,
    pub height: u32,
    pub disposed: bool,
}

impl BitmapData {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            disposed: false,
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
    BITMAPS.insert(BitmapData::new(width, height))
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
