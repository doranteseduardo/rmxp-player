use super::common::{
    bool_to_value, bytes_to_str, define_method, define_singleton_method, float_to_value,
    get_typed_data, install_allocator, ruby_string_bytes, wrap_typed_data, DataTypeBuilder,
    StaticDataType,
};
use crate::native::{value_to_f32, ColorData};
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

const COLOR_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Color\0") };
const COLOR_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Color\0") };

static COLOR_TYPE: StaticDataType =
    StaticDataType::new(|| DataTypeBuilder::new(COLOR_STRUCT_NAME).free(color_free));
static COLOR_CLASS: OnceCell<VALUE> = OnceCell::new();

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
static METHOD_ALPHA: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"alpha\0") });
static METHOD_ALPHA_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"alpha=\0") });
static METHOD_EQUAL: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"==\0") });
static METHOD_DUP: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"dup\0") });
static METHOD_CLONE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"clone\0") });
static METHOD_TO_A: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"to_a\0") });
static METHOD_DUMP: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"_dump\0") });
static METHOD_LOAD: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"_load\0") });

#[derive(Clone)]
struct ColorValue {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

impl Default for ColorValue {
    fn default() -> Self {
        Self {
            red: 0.0,
            green: 0.0,
            blue: 0.0,
            alpha: 255.0,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(COLOR_CLASS_NAME, None);
        let _ = COLOR_CLASS.set(klass);
        install_allocator(klass, Some(color_allocate));
        define_method(klass, *METHOD_INITIALIZE, color_initialize, -1);
        define_method(klass, *METHOD_SET, color_set, -1);
        define_method(klass, *METHOD_RED, color_get_red, -1);
        define_method(klass, *METHOD_RED_SET, color_set_red, -1);
        define_method(klass, *METHOD_GREEN, color_get_green, -1);
        define_method(klass, *METHOD_GREEN_SET, color_set_green, -1);
        define_method(klass, *METHOD_BLUE, color_get_blue, -1);
        define_method(klass, *METHOD_BLUE_SET, color_set_blue, -1);
        define_method(klass, *METHOD_ALPHA, color_get_alpha, -1);
        define_method(klass, *METHOD_ALPHA_SET, color_set_alpha, -1);
        define_method(klass, *METHOD_EQUAL, color_equal, -1);
        define_method(klass, *METHOD_DUP, color_dup, -1);
        define_method(klass, *METHOD_CLONE, color_dup, -1);
        define_method(klass, *METHOD_TO_A, color_to_a, -1);
        define_method(klass, *METHOD_DUMP, color_dump, -1);
        define_singleton_method(klass, *METHOD_LOAD, color_load, -1);
    }
    Ok(())
}

unsafe extern "C" fn color_allocate(klass: VALUE) -> VALUE {
    color_allocate_internal(klass)
}

unsafe fn color_allocate_internal(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, ColorValue::default(), COLOR_TYPE.as_rb_type())
}

unsafe extern "C" fn color_free(ptr: *mut c_void) {
    drop(Box::<ColorValue>::from_raw(ptr as *mut ColorValue));
}

fn get_color_mut(value: VALUE) -> &'static mut ColorValue {
    unsafe { get_typed_data(value, COLOR_TYPE.as_rb_type()) }.expect("Color missing native data")
}

pub fn new_color(red: f32, green: f32, blue: f32, alpha: f32) -> VALUE {
    unsafe {
        let klass = *COLOR_CLASS.get().expect("Color not initialised");
        let value = color_allocate_internal(klass);
        let data = get_color_mut(value);
        data.red = red;
        data.green = green;
        data.blue = blue;
        data.alpha = alpha;
        value
    }
}

pub fn clone_color(value: VALUE) -> VALUE {
    unsafe {
        let klass = rb_obj_class(value);
        let new_value = color_allocate_internal(klass);
        let source = get_color_mut(value).clone();
        *get_color_mut(new_value) = source;
        new_value
    }
}

unsafe extern "C" fn color_initialize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    update_color(self_value, args);
    self_value
}

unsafe extern "C" fn color_set(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    update_color(self_value, args);
    self_value
}

fn update_color(obj: VALUE, args: &[VALUE]) {
    let mut red = 0.0;
    let mut green = 0.0;
    let mut blue = 0.0;
    let mut alpha = 255.0;
    if !args.is_empty() {
        red = clamp_component(value_to_f32(args[0]));
    }
    if args.len() >= 2 {
        green = clamp_component(value_to_f32(args[1]));
    }
    if args.len() >= 3 {
        blue = clamp_component(value_to_f32(args[2]));
    }
    if args.len() >= 4 {
        alpha = clamp_component(value_to_f32(args[3]));
    }
    let color = get_color_mut(obj);
    color.red = red;
    color.green = green;
    color.blue = blue;
    color.alpha = alpha;
}

fn clamp_component(value: f32) -> f32 {
    value.clamp(0.0, 255.0)
}

macro_rules! component_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            float_to_value(get_color_mut(self_value).$field as f64)
        }
    };
}

macro_rules! component_setter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = *argv;
            get_color_mut(self_value).$field = clamp_component(value_to_f32(value));
            value
        }
    };
}

component_getter!(color_get_red, red);
component_getter!(color_get_green, green);
component_getter!(color_get_blue, blue);
component_getter!(color_get_alpha, alpha);

component_setter!(color_set_red, red);
component_setter!(color_set_green, green);
component_setter!(color_set_blue, blue);
component_setter!(color_set_alpha, alpha);

unsafe extern "C" fn color_to_a(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let color = get_color_mut(self_value).clone();
    let array = rb_ary_new_capa(4);
    rb_ary_push(array, float_to_value(color.red as f64));
    rb_ary_push(array, float_to_value(color.green as f64));
    rb_ary_push(array, float_to_value(color.blue as f64));
    rb_ary_push(array, float_to_value(color.alpha as f64));
    array
}

unsafe extern "C" fn color_equal(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, 1)
    };
    if let Some(other) = args.first() {
        if let Some(other_value) = get_typed_data::<ColorValue>(*other, COLOR_TYPE.as_rb_type()) {
            let me = get_color_mut(self_value).clone();
            if me.red == other_value.red
                && me.green == other_value.green
                && me.blue == other_value.blue
                && me.alpha == other_value.alpha
            {
                return bool_to_value(true);
            }
        }
    }
    bool_to_value(false)
}

unsafe extern "C" fn color_dup(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let klass = unsafe { rb_obj_class(self_value) };
    let new_obj = color_allocate(klass);
    let source = get_color_mut(self_value).clone();
    let target = get_color_mut(new_obj);
    *target = source;
    new_obj
}

unsafe extern "C" fn color_dump(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let c = get_color_mut(self_value);
    let mut buf = [0u8; 32];
    buf[0..8].copy_from_slice(&(c.red as f64).to_le_bytes());
    buf[8..16].copy_from_slice(&(c.green as f64).to_le_bytes());
    buf[16..24].copy_from_slice(&(c.blue as f64).to_le_bytes());
    buf[24..32].copy_from_slice(&(c.alpha as f64).to_le_bytes());
    bytes_to_str(&buf)
}

unsafe extern "C" fn color_load(argc: c_int, argv: *const VALUE, klass: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let bytes = match ruby_string_bytes(args[0]) {
        Some(b) if b.len() >= 32 => b,
        _ => return color_allocate_internal(klass),
    };
    let r = f64::from_le_bytes(bytes[0..8].try_into().unwrap()) as f32;
    let g = f64::from_le_bytes(bytes[8..16].try_into().unwrap()) as f32;
    let b = f64::from_le_bytes(bytes[16..24].try_into().unwrap()) as f32;
    let a = f64::from_le_bytes(bytes[24..32].try_into().unwrap()) as f32;
    let obj = color_allocate_internal(klass);
    let data = get_color_mut(obj);
    data.red = clamp_component(r);
    data.green = clamp_component(g);
    data.blue = clamp_component(b);
    data.alpha = clamp_component(a);
    obj
}

pub fn is_color(value: VALUE) -> bool {
    unsafe { get_typed_data::<ColorValue>(value, COLOR_TYPE.as_rb_type()).is_some() }
}

pub fn get_color_data(value: VALUE) -> ColorData {
    if let Some(color) = unsafe { get_typed_data::<ColorValue>(value, COLOR_TYPE.as_rb_type()) } {
        ColorData {
            red: color.red,
            green: color.green,
            blue: color.blue,
            alpha: color.alpha,
        }
    } else {
        ColorData::default()
    }
}
