use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{
    rb_define_module, rb_int2big, rb_num2long, ruby_special_consts, special_consts, VALUE,
};
use std::{
    os::raw::{c_char, c_int},
    sync::atomic::{AtomicI64, AtomicU32, Ordering},
};
static GRAPHICS_MODULE: OnceCell<()> = OnceCell::new();
static FRAME_COUNT: AtomicI64 = AtomicI64::new(0);
static FRAME_RATE: AtomicU32 = AtomicU32::new(60);

const GRAPHICS_NAME: &[u8] = b"Graphics\0";
const UPDATE_NAME: &[u8] = b"update\0";
const FRAME_COUNT_NAME: &[u8] = b"frame_count\0";
const FRAME_COUNT_SET_NAME: &[u8] = b"frame_count=\0";
const FRAME_RATE_NAME: &[u8] = b"frame_rate\0";
const FRAME_RATE_SET_NAME: &[u8] = b"frame_rate=\0";

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
}

pub fn init() -> Result<()> {
    GRAPHICS_MODULE
        .get_or_try_init(|| unsafe { define_graphics() })
        .map(|_| ())
}

#[allow(dead_code)]
pub fn set_frame_rate(rate: u32) {
    FRAME_RATE.store(rate, Ordering::Relaxed);
}

#[allow(dead_code)]
pub fn frame_count() -> i64 {
    FRAME_COUNT.load(Ordering::Relaxed)
}

unsafe fn define_graphics() -> Result<()> {
    let module = rb_define_module(c_name(GRAPHICS_NAME));
    if module == 0 {
        return Err(anyhow!("failed to define Graphics module"));
    }

    rb_define_module_function(module, c_name(UPDATE_NAME), Some(graphics_update), -1);
    rb_define_module_function(
        module,
        c_name(FRAME_COUNT_NAME),
        Some(graphics_get_frame_count),
        -1,
    );
    rb_define_module_function(
        module,
        c_name(FRAME_COUNT_SET_NAME),
        Some(graphics_set_frame_count),
        -1,
    );
    rb_define_module_function(
        module,
        c_name(FRAME_RATE_NAME),
        Some(graphics_get_frame_rate),
        -1,
    );
    rb_define_module_function(
        module,
        c_name(FRAME_RATE_SET_NAME),
        Some(graphics_set_frame_rate),
        -1,
    );
    Ok(())
}

unsafe extern "C" fn graphics_update(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    FRAME_COUNT.fetch_add(1, Ordering::SeqCst);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_frame_count(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    int_to_value(FRAME_COUNT.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_frame_count(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = rb_num2long(*argv);
    FRAME_COUNT.store(value as i64, Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_frame_rate(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    int_to_value(FRAME_RATE.load(Ordering::Relaxed) as i64)
}

unsafe extern "C" fn graphics_set_frame_rate(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = rb_num2long(*argv);
    if value > 0 {
        FRAME_RATE.store(value as u32, Ordering::Relaxed);
    }
    rb_sys::Qnil as VALUE
}

fn int_to_value(value: i64) -> VALUE {
    unsafe {
        if value >= special_consts::FIXNUM_MIN as i64 && value <= special_consts::FIXNUM_MAX as i64
        {
            ((value << ruby_special_consts::RUBY_SPECIAL_SHIFT as i64)
                | ruby_special_consts::RUBY_FIXNUM_FLAG as i64) as VALUE
        } else {
            rb_int2big(value as isize)
        }
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
