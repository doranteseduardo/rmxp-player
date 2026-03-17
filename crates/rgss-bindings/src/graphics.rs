use crate::native;
use anyhow::{anyhow, Result};
use image::RgbaImage;
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    rb_ary_new, rb_ary_push, rb_define_module, rb_float_new, rb_int2big, rb_num2long,
    ruby_special_consts, special_consts, VALUE,
};
use std::{
    os::raw::{c_char, c_int},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, Ordering},
        Arc, RwLock,
    },
};

static GRAPHICS_MODULE: OnceCell<()> = OnceCell::new();
static FRAME_COUNT: AtomicI64 = AtomicI64::new(0);
static FRAME_RATE: AtomicU32 = AtomicU32::new(60);
static SCREEN_WIDTH: AtomicU32 = AtomicU32::new(640);
static SCREEN_HEIGHT: AtomicU32 = AtomicU32::new(480);
static SCREEN_FROZEN: AtomicBool = AtomicBool::new(false);
static LAST_FRAME: Lazy<RwLock<Option<Arc<RgbaImage>>>> = Lazy::new(|| RwLock::new(None));
static FROZEN_FRAME: Lazy<RwLock<Option<Arc<RgbaImage>>>> = Lazy::new(|| RwLock::new(None));
static SCREEN_TONE: Lazy<RwLock<ToneState>> = Lazy::new(|| RwLock::new(ToneState::default()));
static BRIGHTNESS: AtomicI32 = AtomicI32::new(255);
static FLASH_STATE: Lazy<RwLock<Option<FlashState>>> = Lazy::new(|| RwLock::new(None));

const GRAPHICS_NAME: &[u8] = b"Graphics\0";
const UPDATE_NAME: &[u8] = b"update\0";
const FRAME_COUNT_NAME: &[u8] = b"frame_count\0";
const FRAME_COUNT_SET_NAME: &[u8] = b"frame_count=\0";
const FRAME_RATE_NAME: &[u8] = b"frame_rate\0";
const FRAME_RATE_SET_NAME: &[u8] = b"frame_rate=\0";
const FREEZE_NAME: &[u8] = b"freeze\0";
const TRANSITION_NAME: &[u8] = b"transition\0";
const FRAME_RESET_NAME: &[u8] = b"frame_reset\0";
const WAIT_NAME: &[u8] = b"wait\0";
const WIDTH_NAME: &[u8] = b"width\0";
const HEIGHT_NAME: &[u8] = b"height\0";
const RESIZE_SCREEN_NAME: &[u8] = b"resize_screen\0";
const SNAP_TO_BITMAP_NAME: &[u8] = b"_snap_to_bitmap_handle\0";
const BRIGHTNESS_NAME: &[u8] = b"_brightness_value\0";
const BRIGHTNESS_SET_NAME: &[u8] = b"_set_brightness\0";
const TONE_GET_NAME: &[u8] = b"_tone_vector\0";
const TONE_SET_NAME: &[u8] = b"_set_tone\0";
const FLASH_NAME: &[u8] = b"_flash\0";

#[derive(Clone, Copy)]
struct ToneState {
    red: f32,
    green: f32,
    blue: f32,
    gray: f32,
}

impl Default for ToneState {
    fn default() -> Self {
        Self {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            gray: 0.0,
        }
    }
}

impl ToneState {
    fn as_array(self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.gray]
    }
}

#[derive(Clone)]
struct FlashState {
    color: [f32; 4],
    duration: u32,
    remaining: u32,
}

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

pub fn set_screen_size(width: u32, height: u32) {
    SCREEN_WIDTH.store(width.max(1), Ordering::Relaxed);
    SCREEN_HEIGHT.store(height.max(1), Ordering::Relaxed);
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
    rb_define_module_function(module, c_name(FREEZE_NAME), Some(graphics_freeze), 0);
    rb_define_module_function(
        module,
        c_name(TRANSITION_NAME),
        Some(graphics_transition),
        -1,
    );
    rb_define_module_function(
        module,
        c_name(FRAME_RESET_NAME),
        Some(graphics_frame_reset),
        0,
    );
    rb_define_module_function(module, c_name(WAIT_NAME), Some(graphics_wait), -1);
    rb_define_module_function(module, c_name(WIDTH_NAME), Some(graphics_get_width), 0);
    rb_define_module_function(module, c_name(HEIGHT_NAME), Some(graphics_get_height), 0);
    rb_define_module_function(
        module,
        c_name(RESIZE_SCREEN_NAME),
        Some(graphics_resize_screen),
        2,
    );
    rb_define_module_function(
        module,
        c_name(SNAP_TO_BITMAP_NAME),
        Some(graphics_snap_to_bitmap),
        0,
    );
    rb_define_module_function(
        module,
        c_name(BRIGHTNESS_NAME),
        Some(graphics_get_brightness),
        0,
    );
    rb_define_module_function(
        module,
        c_name(BRIGHTNESS_SET_NAME),
        Some(graphics_set_brightness),
        1,
    );
    rb_define_module_function(module, c_name(TONE_GET_NAME), Some(graphics_get_tone), 0);
    rb_define_module_function(module, c_name(TONE_SET_NAME), Some(graphics_set_tone), 4);
    rb_define_module_function(module, c_name(FLASH_NAME), Some(graphics_flash), 5);
    Ok(())
}

unsafe extern "C" fn graphics_update(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    FRAME_COUNT.fetch_add(1, Ordering::SeqCst);
    tick_flash();
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

unsafe extern "C" fn graphics_freeze(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Some(frame) = clone_last_frame() {
        let _ = FROZEN_FRAME.write().map(|mut slot| *slot = Some(frame));
    }
    SCREEN_FROZEN.store(true, Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_transition(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    // Parameters: duration, filename, vague. We currently treat as no-op.
    if argc > 0 && !argv.is_null() {
        let duration = rb_num2long(*argv).max(0);
        FRAME_COUNT.fetch_add(duration as i64, Ordering::Relaxed);
    }
    SCREEN_FROZEN.store(false, Ordering::Relaxed);
    let _ = FROZEN_FRAME.write().map(|mut slot| *slot = None);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_frame_reset(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    FRAME_COUNT.store(0, Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_wait(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let frames = if argc >= 1 && !argv.is_null() {
        let value = rb_num2long(*argv);
        value.max(0)
    } else {
        1
    };
    FRAME_COUNT.fetch_add(frames as i64, Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_width(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    int_to_value(SCREEN_WIDTH.load(Ordering::Relaxed) as i64)
}

unsafe extern "C" fn graphics_get_height(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    int_to_value(SCREEN_HEIGHT.load(Ordering::Relaxed) as i64)
}

unsafe extern "C" fn graphics_resize_screen(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc < 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let width = rb_num2long(*argv) as i64;
    let height = rb_num2long(*argv.add(1)) as i64;
    if width > 0 && height > 0 {
        SCREEN_WIDTH.store(width as u32, Ordering::Relaxed);
        SCREEN_HEIGHT.store(height as u32, Ordering::Relaxed);
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

pub fn store_backbuffer(image: Arc<RgbaImage>) {
    if let Ok(mut slot) = LAST_FRAME.write() {
        *slot = Some(image);
    }
}

fn clone_last_frame() -> Option<Arc<RgbaImage>> {
    LAST_FRAME
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().cloned())
}

#[derive(Clone, Copy)]
pub struct ScreenEffects {
    pub tone: [f32; 4],
    pub brightness: f32,
    pub flash: Option<[f32; 4]>,
}

pub fn screen_effects() -> ScreenEffects {
    let tone_state = SCREEN_TONE
        .read()
        .ok()
        .map(|guard| *guard)
        .unwrap_or_else(ToneState::default);
    let brightness = (BRIGHTNESS.load(Ordering::Relaxed).clamp(0, 255) as f32) / 255.0;
    let flash = current_flash();
    ScreenEffects {
        tone: tone_state.as_array(),
        brightness,
        flash,
    }
}

unsafe extern "C" fn graphics_snap_to_bitmap(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let snapshot = LAST_FRAME
        .read()
        .ok()
        .and_then(|guard| guard.as_ref().cloned());
    if let Some(frame) = snapshot {
        let id = native::create_from_texture(frame);
        rb_sys::rb_uint2inum(id as usize)
    } else {
        rb_sys::Qnil as VALUE
    }
}

unsafe extern "C" fn graphics_get_brightness(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    int_to_value(BRIGHTNESS.load(Ordering::Relaxed) as i64)
}

unsafe extern "C" fn graphics_set_brightness(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = rb_num2long(*argv).clamp(0, 255) as i32;
    BRIGHTNESS.store(value, Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_tone(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let tone = SCREEN_TONE
        .read()
        .ok()
        .map(|guard| *guard)
        .unwrap_or_else(ToneState::default);
    let ary = rb_ary_new();
    rb_ary_push(ary, rb_float_new(tone.red as f64));
    rb_ary_push(ary, rb_float_new(tone.green as f64));
    rb_ary_push(ary, rb_float_new(tone.blue as f64));
    rb_ary_push(ary, rb_float_new(tone.gray as f64));
    ary
}

unsafe extern "C" fn graphics_set_tone(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 4 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 4);
    let mut tone = ToneState::default();
    tone.red = rb_num2long(args[0]) as f32;
    tone.green = rb_num2long(args[1]) as f32;
    tone.blue = rb_num2long(args[2]) as f32;
    tone.gray = rb_num2long(args[3]).clamp(0, 255) as f32;
    if let Ok(mut slot) = SCREEN_TONE.write() {
        *slot = tone;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_flash(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 5 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let color = [
        rb_num2long(args[0]).clamp(0, 255) as f32,
        rb_num2long(args[1]).clamp(0, 255) as f32,
        rb_num2long(args[2]).clamp(0, 255) as f32,
        rb_num2long(args[3]).clamp(0, 255) as f32,
    ];
    let duration = rb_num2long(args[4]).max(0) as u32;
    if let Ok(mut slot) = FLASH_STATE.write() {
        if duration == 0 {
            *slot = None;
        } else {
            let state = FlashState {
                color,
                duration,
                remaining: duration,
            };
            *slot = Some(state);
        }
    }
    rb_sys::Qnil as VALUE
}

fn tick_flash() {
    if let Ok(mut slot) = FLASH_STATE.write() {
        if let Some(state) = slot.as_mut() {
            if state.remaining > 0 {
                state.remaining -= 1;
                if state.remaining == 0 {
                    *slot = None;
                }
            }
        }
    }
}

fn current_flash() -> Option<[f32; 4]> {
    FLASH_STATE.read().ok().and_then(|state| {
        if let Some(flash) = state.as_ref() {
            if flash.remaining == 0 || flash.duration == 0 {
                return None;
            } else {
                let strength = flash.remaining as f32 / flash.duration as f32;
                let mut color = flash.color;
                color[3] *= strength;
                return Some(color);
            }
        }
        None
    })
}
