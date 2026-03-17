use crate::{native, runtime, system};
use anyhow::{anyhow, Result};
use image::RgbaImage;
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    rb_ary_new, rb_ary_push, rb_cObject, rb_const_get, rb_define_module, rb_eRuntimeError,
    rb_float_new, rb_int2big, rb_intern, rb_num2long, rb_raise, ruby_special_consts,
    special_consts, VALUE,
};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, Ordering},
        Arc, RwLock,
    },
};
use tracing::warn;

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
static BLUR_REQUEST: AtomicBool = AtomicBool::new(false);
static SHARPEN_REQUEST: AtomicBool = AtomicBool::new(false);
static FLASH_STATE: Lazy<RwLock<Option<FlashState>>> = Lazy::new(|| RwLock::new(None));
static FADE_STATE: Lazy<RwLock<Option<FadeState>>> = Lazy::new(|| RwLock::new(None));
static FULLSCREEN: AtomicBool = AtomicBool::new(false);
static SHOW_CURSOR: AtomicBool = AtomicBool::new(true);
static SCALE_FACTOR: AtomicU32 = AtomicU32::new(1.0f32.to_bits());
static FRAMESKIP: AtomicBool = AtomicBool::new(false);
static FIXED_ASPECT: AtomicBool = AtomicBool::new(true);
static SMOOTH_SCALING: AtomicI32 = AtomicI32::new(0);
static INTEGER_SCALING: AtomicBool = AtomicBool::new(false);
static LAST_MILE_SCALING: AtomicBool = AtomicBool::new(false);
static THREAD_SAFE: AtomicBool = AtomicBool::new(false);

static HANGUP_REQUESTED: AtomicBool = AtomicBool::new(false);
static HANGUP_CLASS: OnceCell<VALUE> = OnceCell::new();
static HANGUP_MESSAGE: Lazy<CString> =
    Lazy::new(|| CString::new("window closed").expect("CString literal"));

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
const FADEOUT_NAME: &[u8] = b"fadeout\0";
const FADEIN_NAME: &[u8] = b"fadein\0";
const WIDTH_NAME: &[u8] = b"width\0";
const HEIGHT_NAME: &[u8] = b"height\0";
const RESIZE_SCREEN_NAME: &[u8] = b"resize_screen\0";
const SNAP_TO_BITMAP_NAME: &[u8] = b"_snap_to_bitmap_handle\0";
const SCREENSHOT_NAME: &[u8] = b"screenshot\0";
const BRIGHTNESS_NAME: &[u8] = b"_brightness_value\0";
const BRIGHTNESS_SET_NAME: &[u8] = b"_set_brightness\0";
const TONE_GET_NAME: &[u8] = b"_tone_vector\0";
const TONE_SET_NAME: &[u8] = b"_set_tone\0";
const FLASH_NAME: &[u8] = b"_flash\0";
const DISPLAY_WIDTH_NAME: &[u8] = b"display_width\0";
const DISPLAY_HEIGHT_NAME: &[u8] = b"display_height\0";
const CENTER_NAME: &[u8] = b"center\0";
const RESIZE_WINDOW_NAME: &[u8] = b"resize_window\0";
const BLUR_NAME: &[u8] = b"blur\0";
const SHARPEN_NAME: &[u8] = b"sharpen\0";
const DELTA_NAME: &[u8] = b"delta\0";
const AVERAGE_FRAME_RATE_NAME: &[u8] = b"average_frame_rate\0";
const FULLSCREEN_NAME: &[u8] = b"fullscreen\0";
const FULLSCREEN_SET_NAME: &[u8] = b"fullscreen=\0";
const SHOW_CURSOR_NAME: &[u8] = b"show_cursor\0";
const SHOW_CURSOR_SET_NAME: &[u8] = b"show_cursor=\0";
const SCALE_NAME: &[u8] = b"scale\0";
const SCALE_SET_NAME: &[u8] = b"scale=\0";
const FRAMESKIP_NAME: &[u8] = b"frameskip\0";
const FRAMESKIP_SET_NAME: &[u8] = b"frameskip=\0";
const FIXED_ASPECT_NAME: &[u8] = b"fixed_aspect_ratio\0";
const FIXED_ASPECT_SET_NAME: &[u8] = b"fixed_aspect_ratio=\0";
const SMOOTH_SCALING_NAME: &[u8] = b"smooth_scaling\0";
const SMOOTH_SCALING_SET_NAME: &[u8] = b"smooth_scaling=\0";
const INTEGER_SCALING_NAME: &[u8] = b"integer_scaling\0";
const INTEGER_SCALING_SET_NAME: &[u8] = b"integer_scaling=\0";
const LAST_MILE_SCALING_NAME: &[u8] = b"last_mile_scaling\0";
const LAST_MILE_SCALING_SET_NAME: &[u8] = b"last_mile_scaling=\0";
const THREAD_SAFE_NAME: &[u8] = b"thread_safe\0";
const THREAD_SAFE_SET_NAME: &[u8] = b"thread_safe=\0";

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
    duration: i64,
    remaining: i64,
}

#[derive(Clone, Copy)]
struct FadeState {
    remaining: i64,
    total: i64,
    start: i32,
    end: i32,
}

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
    fn rb_string_value_cstr(value: *mut VALUE) -> *const c_char;
}

pub fn init() -> Result<()> {
    GRAPHICS_MODULE
        .get_or_try_init(|| unsafe { define_graphics() })
        .map(|_| ())
}

pub fn request_hangup() {
    HANGUP_REQUESTED.store(true, Ordering::SeqCst);
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
    rb_define_module_function(module, c_name(FADEOUT_NAME), Some(graphics_fadeout), -1);
    rb_define_module_function(module, c_name(FADEIN_NAME), Some(graphics_fadein), -1);
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
        c_name(SCREENSHOT_NAME),
        Some(graphics_screenshot),
        1,
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
    rb_define_module_function(module, c_name(DELTA_NAME), Some(graphics_delta), 0);
    rb_define_module_function(
        module,
        c_name(DISPLAY_WIDTH_NAME),
        Some(graphics_display_width),
        0,
    );
    rb_define_module_function(
        module,
        c_name(DISPLAY_HEIGHT_NAME),
        Some(graphics_display_height),
        0,
    );
    rb_define_module_function(module, c_name(CENTER_NAME), Some(graphics_center), 0);
    rb_define_module_function(
        module,
        c_name(RESIZE_WINDOW_NAME),
        Some(graphics_resize_window),
        2,
    );
    rb_define_module_function(module, c_name(BLUR_NAME), Some(graphics_blur), 0);
    rb_define_module_function(module, c_name(SHARPEN_NAME), Some(graphics_sharpen), 0);
    rb_define_module_function(
        module,
        c_name(AVERAGE_FRAME_RATE_NAME),
        Some(graphics_average_frame_rate),
        0,
    );
    rb_define_module_function(
        module,
        c_name(FULLSCREEN_NAME),
        Some(graphics_get_fullscreen),
        0,
    );
    rb_define_module_function(
        module,
        c_name(FULLSCREEN_SET_NAME),
        Some(graphics_set_fullscreen),
        1,
    );
    rb_define_module_function(
        module,
        c_name(SHOW_CURSOR_NAME),
        Some(graphics_get_show_cursor),
        0,
    );
    rb_define_module_function(
        module,
        c_name(SHOW_CURSOR_SET_NAME),
        Some(graphics_set_show_cursor),
        1,
    );
    rb_define_module_function(module, c_name(SCALE_NAME), Some(graphics_get_scale), 0);
    rb_define_module_function(module, c_name(SCALE_SET_NAME), Some(graphics_set_scale), 1);
    rb_define_module_function(
        module,
        c_name(FRAMESKIP_NAME),
        Some(graphics_get_frameskip),
        0,
    );
    rb_define_module_function(
        module,
        c_name(FRAMESKIP_SET_NAME),
        Some(graphics_set_frameskip),
        1,
    );
    rb_define_module_function(
        module,
        c_name(FIXED_ASPECT_NAME),
        Some(graphics_get_fixed_aspect),
        0,
    );
    rb_define_module_function(
        module,
        c_name(FIXED_ASPECT_SET_NAME),
        Some(graphics_set_fixed_aspect),
        1,
    );
    rb_define_module_function(
        module,
        c_name(SMOOTH_SCALING_NAME),
        Some(graphics_get_smooth_scaling),
        0,
    );
    rb_define_module_function(
        module,
        c_name(SMOOTH_SCALING_SET_NAME),
        Some(graphics_set_smooth_scaling),
        1,
    );
    rb_define_module_function(
        module,
        c_name(INTEGER_SCALING_NAME),
        Some(graphics_get_integer_scaling),
        0,
    );
    rb_define_module_function(
        module,
        c_name(INTEGER_SCALING_SET_NAME),
        Some(graphics_set_integer_scaling),
        1,
    );
    rb_define_module_function(
        module,
        c_name(LAST_MILE_SCALING_NAME),
        Some(graphics_get_last_mile_scaling),
        0,
    );
    rb_define_module_function(
        module,
        c_name(LAST_MILE_SCALING_SET_NAME),
        Some(graphics_set_last_mile_scaling),
        1,
    );
    rb_define_module_function(
        module,
        c_name(THREAD_SAFE_NAME),
        Some(graphics_get_thread_safe),
        0,
    );
    rb_define_module_function(
        module,
        c_name(THREAD_SAFE_SET_NAME),
        Some(graphics_set_thread_safe),
        1,
    );
    Ok(())
}

unsafe extern "C" fn graphics_update(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if HANGUP_REQUESTED.swap(false, Ordering::SeqCst) {
        raise_hangup();
    }
    if let Err(err) = runtime::yield_frame() {
        warn!(target: "rgss", error = %err, "Graphics.update yield failed");
    }
    advance_time(1);
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
        let duration = rb_num2long(*argv).max(0) as i64;
        advance_time(duration);
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
    advance_time(frames as i64);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_fadeout(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let frames = if argc >= 1 && !argv.is_null() {
        rb_num2long(*argv).max(0)
    } else {
        30
    } as i64;
    start_fade(frames, 0);
    advance_time(frames);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_fadein(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let frames = if argc >= 1 && !argv.is_null() {
        rb_num2long(*argv).max(0)
    } else {
        30
    } as i64;
    start_fade(frames, 255);
    advance_time(frames);
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
        system::resize_window(width as u32, height as u32);
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_resize_window(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    graphics_resize_screen(argc, argv, _self)
}

unsafe extern "C" fn graphics_blur(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if !apply_filter_to_frozen(ScreenFilter::Blur) {
        BLUR_REQUEST.store(true, Ordering::SeqCst);
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_sharpen(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if !apply_filter_to_frozen(ScreenFilter::Sharpen) {
        SHARPEN_REQUEST.store(true, Ordering::SeqCst);
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

fn clamp_tone_channel(value: f32) -> f32 {
    value.clamp(-255.0, 255.0)
}

fn clamp_gray_channel(value: f32) -> f32 {
    value.clamp(0.0, 255.0)
}

#[derive(Clone, Copy)]
enum ScreenFilter {
    Blur,
    Sharpen,
}

fn apply_filter_to_frozen(kind: ScreenFilter) -> bool {
    if !SCREEN_FROZEN.load(Ordering::Relaxed) {
        return false;
    }
    if let Ok(mut slot) = FROZEN_FRAME.write() {
        if let Some(frame) = slot.as_ref() {
            if let Some(filtered) = filter_image(frame, kind) {
                *slot = Some(Arc::new(filtered));
                return true;
            }
        }
    }
    false
}

fn filter_image(image: &RgbaImage, kind: ScreenFilter) -> Option<RgbaImage> {
    let mut data = image.clone().into_vec();
    let size = (image.width(), image.height());
    apply_filter_kernel(size, &mut data, kind);
    RgbaImage::from_raw(size.0, size.1, data)
}

fn apply_filter_kernel(size: (u32, u32), data: &mut [u8], kind: ScreenFilter) {
    let kernel = match kind {
        ScreenFilter::Blur => [
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
            1.0 / 9.0,
        ],
        ScreenFilter::Sharpen => [0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0],
    };
    convolve(size, data, &kernel);
}

fn convolve(size: (u32, u32), data: &mut [u8], kernel: &[f32; 9]) {
    let (width, height) = size;
    if width == 0 || height == 0 {
        return;
    }
    let source = data.to_vec();
    let width_i = width as i32;
    let height_i = height as i32;
    for y in 0..height_i {
        for x in 0..width_i {
            let mut accum = [0.0f32; 3];
            for ky in -1..=1 {
                for kx in -1..=1 {
                    let sample_x = (x + kx).clamp(0, width_i - 1) as u32;
                    let sample_y = (y + ky).clamp(0, height_i - 1) as u32;
                    let idx = ((sample_y * width + sample_x) * 4) as usize;
                    let weight = kernel[((ky + 1) * 3 + (kx + 1)) as usize];
                    for channel in 0..3 {
                        accum[channel] += source[idx + channel] as f32 * weight;
                    }
                }
            }
            let dst_idx = ((y as u32 * width + x as u32) * 4) as usize;
            for channel in 0..3 {
                data[dst_idx + channel] = accum[channel].clamp(0.0, 255.0).round() as u8;
            }
            data[dst_idx + 3] = source[dst_idx + 3];
        }
    }
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
    pub blur: bool,
    pub sharpen: bool,
}

pub fn screen_effects() -> ScreenEffects {
    let tone_state = SCREEN_TONE
        .read()
        .ok()
        .map(|guard| *guard)
        .unwrap_or_else(ToneState::default);
    let brightness = (BRIGHTNESS.load(Ordering::Relaxed).clamp(0, 255) as f32) / 255.0;
    let flash = current_flash();
    let blur = BLUR_REQUEST.swap(false, Ordering::SeqCst);
    let sharpen = SHARPEN_REQUEST.swap(false, Ordering::SeqCst);
    ScreenEffects {
        tone: tone_state.as_array(),
        brightness,
        flash,
        blur,
        sharpen,
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

unsafe extern "C" fn graphics_screenshot(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut arg = *argv;
    let ptr = rb_string_value_cstr(&mut arg);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let path = CStr::from_ptr(ptr).to_string_lossy().to_string();
    match clone_last_frame() {
        Some(texture) => {
            let destination = Path::new(path.as_str());
            let result = texture.save(destination);
            if let Err(err) = result {
                warn!(
                    target: "rgss",
                    path = %path,
                    error = %err,
                    "Failed to write screenshot"
                );
            }
        }
        None => {
            warn!(target: "rgss", path = %path, "No frame to capture for screenshot");
        }
    }
    rb_sys::Qnil as VALUE
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
    tone.red = clamp_tone_channel(rb_num2long(args[0]) as f32);
    tone.green = clamp_tone_channel(rb_num2long(args[1]) as f32);
    tone.blue = clamp_tone_channel(rb_num2long(args[2]) as f32);
    tone.gray = clamp_gray_channel(rb_num2long(args[3]) as f32);
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
    let duration = rb_num2long(args[4]).max(0) as i64;
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

unsafe extern "C" fn graphics_delta(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    rb_float_new(system::current_delta())
}

unsafe extern "C" fn graphics_average_frame_rate(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let delta = system::current_delta();
    if delta <= f64::EPSILON {
        rb_float_new(0.0)
    } else {
        rb_float_new(1.0 / delta)
    }
}

unsafe extern "C" fn graphics_display_width(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let (width, _) = system::display_size();
    int_to_value(width as i64)
}

unsafe extern "C" fn graphics_display_height(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let (_, height) = system::display_size();
    int_to_value(height as i64)
}

unsafe extern "C" fn graphics_center(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    system::center_window();
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_fullscreen(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(FULLSCREEN.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_fullscreen(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let enabled = value_to_bool(*argv);
    FULLSCREEN.store(enabled, Ordering::Relaxed);
    system::set_fullscreen(enabled);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_show_cursor(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(SHOW_CURSOR.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_show_cursor(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let visible = value_to_bool(*argv);
    SHOW_CURSOR.store(visible, Ordering::Relaxed);
    system::set_cursor_visible(visible);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_scale(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    rb_float_new(f32::from_bits(SCALE_FACTOR.load(Ordering::Relaxed)) as f64)
}

unsafe extern "C" fn graphics_set_scale(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = rb_num2long(*argv) as f32;
    let clamped = if value <= 0.0 { 1.0 } else { value };
    SCALE_FACTOR.store(clamped.to_bits(), Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_frameskip(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(FRAMESKIP.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_frameskip(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    FRAMESKIP.store(value_to_bool(*argv), Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_fixed_aspect(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(FIXED_ASPECT.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_fixed_aspect(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    FIXED_ASPECT.store(value_to_bool(*argv), Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_smooth_scaling(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    int_to_value(SMOOTH_SCALING.load(Ordering::Relaxed) as i64)
}

unsafe extern "C" fn graphics_set_smooth_scaling(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = rb_num2long(*argv) as i32;
    SMOOTH_SCALING.store(value, Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_integer_scaling(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(INTEGER_SCALING.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_integer_scaling(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    INTEGER_SCALING.store(value_to_bool(*argv), Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_last_mile_scaling(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(LAST_MILE_SCALING.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_last_mile_scaling(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    LAST_MILE_SCALING.store(value_to_bool(*argv), Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn graphics_get_thread_safe(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(THREAD_SAFE.load(Ordering::Relaxed))
}

unsafe extern "C" fn graphics_set_thread_safe(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    THREAD_SAFE.store(value_to_bool(*argv), Ordering::Relaxed);
    rb_sys::Qnil as VALUE
}

fn current_flash() -> Option<[f32; 4]> {
    FLASH_STATE.read().ok().and_then(|state| {
        if let Some(flash) = state.as_ref() {
            if flash.remaining == 0 || flash.duration == 0 {
                return None;
            } else {
                let strength = flash.remaining.max(0) as f32 / flash.duration.max(1) as f32;
                let mut color = flash.color;
                color[3] *= strength;
                return Some(color);
            }
        }
        None
    })
}

fn advance_time(frames: i64) {
    if frames <= 0 {
        return;
    }
    FRAME_COUNT.fetch_add(frames, Ordering::SeqCst);
    advance_flash(frames);
    advance_fade(frames);
}

fn advance_flash(frames: i64) {
    if frames <= 0 {
        return;
    }
    if let Ok(mut slot) = FLASH_STATE.write() {
        if let Some(state) = slot.as_mut() {
            state.remaining = (state.remaining - frames).max(0);
            if state.remaining == 0 {
                *slot = None;
            }
        }
    }
}

fn advance_fade(frames: i64) {
    if frames <= 0 {
        return;
    }
    if let Ok(mut slot) = FADE_STATE.write() {
        if let Some(fade) = slot.as_mut() {
            fade.remaining = (fade.remaining - frames).max(0);
            let total = fade.total.max(1) as f32;
            let progress = 1.0 - (fade.remaining as f32 / total);
            let interpolated =
                fade.start as f32 + (fade.end - fade.start) as f32 * progress.clamp(0.0, 1.0);
            BRIGHTNESS.store(interpolated.round() as i32, Ordering::Relaxed);
            if fade.remaining == 0 {
                BRIGHTNESS.store(fade.end, Ordering::Relaxed);
                *slot = None;
            }
        }
    }
}

fn start_fade(frames: i64, target: i32) {
    if frames <= 0 {
        BRIGHTNESS.store(target.clamp(0, 255), Ordering::Relaxed);
        if let Ok(mut slot) = FADE_STATE.write() {
            *slot = None;
        }
        return;
    }
    let start = BRIGHTNESS.load(Ordering::Relaxed);
    let total = frames.max(1);
    if let Ok(mut slot) = FADE_STATE.write() {
        *slot = Some(FadeState {
            remaining: total,
            total,
            start,
            end: target.clamp(0, 255),
        });
    }
}

fn bool_to_value(value: bool) -> VALUE {
    if value {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

fn value_to_bool(value: VALUE) -> bool {
    value != rb_sys::Qfalse as VALUE && value != rb_sys::Qnil as VALUE
}

#[allow(unreachable_code)]
fn raise_hangup() -> ! {
    unsafe {
        rb_raise(hangup_class(), HANGUP_MESSAGE.as_ptr());
        std::hint::unreachable_unchecked();
    }
}

fn hangup_class() -> VALUE {
    *HANGUP_CLASS.get_or_init(|| unsafe {
        let name = CString::new("Hangup").expect("CString literal");
        let id = rb_intern(name.as_ptr());
        let class = rb_const_get(rb_cObject, id);
        if class == 0 {
            rb_eRuntimeError
        } else {
            class
        }
    })
}
