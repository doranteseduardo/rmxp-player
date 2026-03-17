use super::common::{
    bool_to_value, define_method, float_to_value, get_typed_data, install_allocator,
    wrap_typed_data, DataTypeBuilder, StaticDataType,
};
use crate::native::{value_to_f32, ToneData};
use anyhow::Result;
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    bindings::{rb_ary_new_capa, rb_ary_push, rb_obj_class},
    VALUE,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_int,
    slice,
};

const TONE_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Tone\0") };
const TONE_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Tone\0") };

static TONE_TYPE: StaticDataType =
    StaticDataType::new(|| DataTypeBuilder::new(TONE_STRUCT_NAME).free(tone_free));
static TONE_CLASS: OnceCell<VALUE> = OnceCell::new();

static METHOD_INITIALIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"initialize\0") });
static METHOD_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"set\0") });
static METHOD_RED: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"red\0") });
static METHOD_RED_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"red=\0") });
static METHOD_GREEN: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"green\0") });
static METHOD_GREEN_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"green=\0") });
static METHOD_BLUE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"blue\0") });
static METHOD_BLUE_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"blue=\0") });
static METHOD_GRAY: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"gray\0") });
static METHOD_GRAY_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"gray=\0") });
static METHOD_EQUAL: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"==\0") });
static METHOD_DUP: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"dup\0") });
static METHOD_TO_A: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"to_a\0") });

#[derive(Clone)]
struct ToneValue {
    red: f32,
    green: f32,
    blue: f32,
    gray: f32,
}

impl Default for ToneValue {
    fn default() -> Self {
        Self {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            gray: 0.0,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(TONE_CLASS_NAME, None);
        let _ = TONE_CLASS.set(klass);
        install_allocator(klass, Some(tone_allocate));
        define_method(klass, *METHOD_INITIALIZE, tone_initialize, -1);
        define_method(klass, *METHOD_SET, tone_set, -1);
        define_method(klass, *METHOD_RED, tone_get_red, 0);
        define_method(klass, *METHOD_RED_SET, tone_set_red, 1);
        define_method(klass, *METHOD_GREEN, tone_get_green, 0);
        define_method(klass, *METHOD_GREEN_SET, tone_set_green, 1);
        define_method(klass, *METHOD_BLUE, tone_get_blue, 0);
        define_method(klass, *METHOD_BLUE_SET, tone_set_blue, 1);
        define_method(klass, *METHOD_GRAY, tone_get_gray, 0);
        define_method(klass, *METHOD_GRAY_SET, tone_set_gray, 1);
        define_method(klass, *METHOD_EQUAL, tone_equal, 1);
        define_method(klass, *METHOD_DUP, tone_dup, 0);
        define_method(klass, *METHOD_TO_A, tone_to_a, 0);
    }
    Ok(())
}

unsafe extern "C" fn tone_allocate(klass: VALUE) -> VALUE {
    tone_allocate_internal(klass)
}

unsafe fn tone_allocate_internal(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, ToneValue::default(), TONE_TYPE.as_rb_type())
}

unsafe extern "C" fn tone_free(ptr: *mut c_void) {
    drop(Box::<ToneValue>::from_raw(ptr as *mut ToneValue));
}

fn get_tone_mut(value: VALUE) -> &'static mut ToneValue {
    unsafe { get_typed_data(value, TONE_TYPE.as_rb_type()) }.expect("Tone missing native data")
}

unsafe extern "C" fn tone_initialize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    update_tone(self_value, args);
    self_value
}

unsafe extern "C" fn tone_set(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    update_tone(self_value, args);
    self_value
}

fn update_tone(obj: VALUE, args: &[VALUE]) {
    let mut value = ToneValue::default();
    if !args.is_empty() {
        value.red = clamp_rgb(value_to_f32(args[0]));
    }
    if args.len() >= 2 {
        value.green = clamp_rgb(value_to_f32(args[1]));
    }
    if args.len() >= 3 {
        value.blue = clamp_rgb(value_to_f32(args[2]));
    }
    if args.len() >= 4 {
        value.gray = clamp_gray(value_to_f32(args[3]));
    }
    let tone = get_tone_mut(obj);
    *tone = value;
}

fn clamp_rgb(value: f32) -> f32 {
    value.clamp(-255.0, 255.0)
}

fn clamp_gray(value: f32) -> f32 {
    value.clamp(0.0, 255.0)
}

macro_rules! tone_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            float_to_value(get_tone_mut(self_value).$field as f64)
        }
    };
}

macro_rules! tone_setter {
    ($name:ident, $field:ident, $clamp:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = *argv;
            get_tone_mut(self_value).$field = $clamp(value_to_f32(value));
            value
        }
    };
}

tone_getter!(tone_get_red, red);
tone_getter!(tone_get_green, green);
tone_getter!(tone_get_blue, blue);
tone_getter!(tone_get_gray, gray);

tone_setter!(tone_set_red, red, clamp_rgb);
tone_setter!(tone_set_green, green, clamp_rgb);
tone_setter!(tone_set_blue, blue, clamp_rgb);
tone_setter!(tone_set_gray, gray, clamp_gray);

unsafe extern "C" fn tone_equal(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, 1)
    };
    if let Some(other) = args.first() {
        if let Some(other_value) = get_typed_data::<ToneValue>(*other, TONE_TYPE.as_rb_type()) {
            let me = get_tone_mut(self_value).clone();
            if me.red == other_value.red
                && me.green == other_value.green
                && me.blue == other_value.blue
                && me.gray == other_value.gray
            {
                return bool_to_value(true);
            }
        }
    }
    bool_to_value(false)
}

unsafe extern "C" fn tone_dup(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let klass = rb_obj_class(self_value);
    let new_obj = tone_allocate_internal(klass);
    let source = get_tone_mut(self_value).clone();
    let target = get_tone_mut(new_obj);
    *target = source;
    new_obj
}

unsafe extern "C" fn tone_to_a(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let tone = get_tone_mut(self_value).clone();
    let array = rb_ary_new_capa(4);
    rb_ary_push(array, float_to_value(tone.red as f64));
    rb_ary_push(array, float_to_value(tone.green as f64));
    rb_ary_push(array, float_to_value(tone.blue as f64));
    rb_ary_push(array, float_to_value(tone.gray as f64));
    array
}

pub fn new_tone(red: f32, green: f32, blue: f32, gray: f32) -> VALUE {
    unsafe {
        let klass = *TONE_CLASS.get().expect("Tone not initialised");
        let value = tone_allocate_internal(klass);
        let tone = get_tone_mut(value);
        tone.red = red;
        tone.green = green;
        tone.blue = blue;
        tone.gray = gray;
        value
    }
}

pub fn clone_tone(value: VALUE) -> VALUE {
    unsafe {
        let klass = rb_obj_class(value);
        let new_value = tone_allocate_internal(klass);
        let source = get_tone_mut(value).clone();
        *get_tone_mut(new_value) = source;
        new_value
    }
}

pub fn is_tone(value: VALUE) -> bool {
    unsafe { get_typed_data::<ToneValue>(value, TONE_TYPE.as_rb_type()).is_some() }
}

pub fn tone_data(value: VALUE) -> ToneData {
    if let Some(tone) = unsafe { get_typed_data::<ToneValue>(value, TONE_TYPE.as_rb_type()) } {
        ToneData::new(tone.red, tone.green, tone.blue, tone.gray)
    } else {
        ToneData::default()
    }
}
