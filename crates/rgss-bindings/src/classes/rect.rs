use super::common::{
    bool_to_value, bytes_to_str, define_method, define_singleton_method, get_typed_data,
    install_allocator, int_to_value, ruby_string_bytes, wrap_typed_data, DataTypeBuilder,
    StaticDataType,
};
use crate::native::{value_to_i32, RectData};
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

const RECT_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Rect\0") };
const RECT_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Rect\0") };

static RECT_TYPE: StaticDataType =
    StaticDataType::new(|| DataTypeBuilder::new(RECT_STRUCT_NAME).free(rect_free));
static RECT_CLASS: OnceCell<VALUE> = OnceCell::new();

static METHOD_INITIALIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"initialize\0") });
static METHOD_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"set\0") });
static METHOD_EMPTY: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"empty\0") });
static METHOD_DUP: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"dup\0") });
static METHOD_CLONE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"clone\0") });
static METHOD_EQUAL: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"==\0") });
static METHOD_TO_A: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"to_a\0") });
static METHOD_DUMP: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"_dump\0") });
static METHOD_LOAD: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"_load\0") });
static METHOD_X: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"x\0") });
static METHOD_X_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"x=\0") });
static METHOD_Y: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"y\0") });
static METHOD_Y_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"y=\0") });
static METHOD_WIDTH: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"width\0") });
static METHOD_WIDTH_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"width=\0") });
static METHOD_HEIGHT: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"height\0") });
static METHOD_HEIGHT_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"height=\0") });

#[derive(Clone)]
struct RectValue {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Default for RectValue {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(RECT_CLASS_NAME, None);
        let _ = RECT_CLASS.set(klass);
        install_allocator(klass, Some(rect_allocate));
        define_method(klass, *METHOD_INITIALIZE, rect_initialize, -1);
        define_method(klass, *METHOD_SET, rect_set, -1);
        define_method(klass, *METHOD_X, rect_get_x, -1);
        define_method(klass, *METHOD_X_SET, rect_set_x, -1);
        define_method(klass, *METHOD_Y, rect_get_y, -1);
        define_method(klass, *METHOD_Y_SET, rect_set_y, -1);
        define_method(klass, *METHOD_WIDTH, rect_get_width, -1);
        define_method(klass, *METHOD_WIDTH_SET, rect_set_width, -1);
        define_method(klass, *METHOD_HEIGHT, rect_get_height, -1);
        define_method(klass, *METHOD_HEIGHT_SET, rect_set_height, -1);
        define_method(klass, *METHOD_EMPTY, rect_empty, -1);
        define_method(klass, *METHOD_DUP, rect_dup, -1);
        define_method(klass, *METHOD_CLONE, rect_dup, -1);
        define_method(klass, *METHOD_EQUAL, rect_equal, -1);
        define_method(klass, *METHOD_TO_A, rect_to_a, -1);
        define_method(klass, *METHOD_DUMP, rect_dump, -1);
        define_singleton_method(klass, *METHOD_LOAD, rect_load, -1);
    }
    Ok(())
}

unsafe extern "C" fn rect_allocate(klass: VALUE) -> VALUE {
    rect_allocate_internal(klass)
}

unsafe fn rect_allocate_internal(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, RectValue::default(), RECT_TYPE.as_rb_type())
}

unsafe extern "C" fn rect_free(ptr: *mut c_void) {
    drop(Box::<RectValue>::from_raw(ptr as *mut RectValue));
}

fn get_rect_mut(value: VALUE) -> &'static mut RectValue {
    unsafe { get_typed_data(value, RECT_TYPE.as_rb_type()) }.expect("Rect missing native data")
}

unsafe extern "C" fn rect_initialize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    apply_rect(self_value, argc, argv);
    self_value
}

unsafe extern "C" fn rect_set(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    apply_rect(self_value, argc, argv);
    self_value
}

unsafe fn apply_rect(obj: VALUE, argc: c_int, argv: *const VALUE) {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let rect = get_rect_mut(obj);
    rect.x = args.get(0).map(|v| value_to_i32(*v)).unwrap_or(0);
    rect.y = args.get(1).map(|v| value_to_i32(*v)).unwrap_or(0);
    rect.width = args.get(2).map(|v| value_to_i32(*v)).unwrap_or(0);
    rect.height = args.get(3).map(|v| value_to_i32(*v)).unwrap_or(0);
}

macro_rules! rect_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            int_to_value(get_rect_mut(self_value).$field as i64)
        }
    };
}

macro_rules! rect_setter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = *argv;
            get_rect_mut(self_value).$field = value_to_i32(value);
            value
        }
    };
}

rect_getter!(rect_get_x, x);
rect_getter!(rect_get_y, y);
rect_getter!(rect_get_width, width);
rect_getter!(rect_get_height, height);

rect_setter!(rect_set_x, x);
rect_setter!(rect_set_y, y);
rect_setter!(rect_set_width, width);
rect_setter!(rect_set_height, height);

unsafe extern "C" fn rect_empty(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let rect = get_rect_mut(self_value);
    rect.x = 0;
    rect.y = 0;
    rect.width = 0;
    rect.height = 0;
    self_value
}

unsafe extern "C" fn rect_dup(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let klass = rb_obj_class(self_value);
    let new_obj = rect_allocate_internal(klass);
    let source = get_rect_mut(self_value).clone();
    let target = get_rect_mut(new_obj);
    *target = source;
    new_obj
}

unsafe extern "C" fn rect_equal(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return bool_to_value(false);
    }
    let other = *argv;
    if let Some(other_rect) = get_typed_data::<RectValue>(other, RECT_TYPE.as_rb_type()) {
        let me = get_rect_mut(self_value);
        return bool_to_value(
            me.x == other_rect.x
                && me.y == other_rect.y
                && me.width == other_rect.width
                && me.height == other_rect.height,
        );
    }
    bool_to_value(false)
}

unsafe extern "C" fn rect_to_a(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let rect = get_rect_mut(self_value).clone();
    let array = rb_ary_new_capa(4);
    rb_ary_push(array, int_to_value(rect.x as i64));
    rb_ary_push(array, int_to_value(rect.y as i64));
    rb_ary_push(array, int_to_value(rect.width as i64));
    rb_ary_push(array, int_to_value(rect.height as i64));
    array
}

unsafe extern "C" fn rect_dump(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let r = get_rect_mut(self_value);
    let mut buf = [0u8; 32];
    buf[0..8].copy_from_slice(&(r.x as f64).to_le_bytes());
    buf[8..16].copy_from_slice(&(r.y as f64).to_le_bytes());
    buf[16..24].copy_from_slice(&(r.width as f64).to_le_bytes());
    buf[24..32].copy_from_slice(&(r.height as f64).to_le_bytes());
    bytes_to_str(&buf)
}

unsafe extern "C" fn rect_load(argc: c_int, argv: *const VALUE, klass: VALUE) -> VALUE {
    if argc <= 0 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let bytes = match ruby_string_bytes(args[0]) {
        Some(b) if b.len() >= 32 => b,
        _ => return rect_allocate_internal(klass),
    };
    let x = f64::from_le_bytes(bytes[0..8].try_into().unwrap()) as i32;
    let y = f64::from_le_bytes(bytes[8..16].try_into().unwrap()) as i32;
    let w = f64::from_le_bytes(bytes[16..24].try_into().unwrap()) as i32;
    let h = f64::from_le_bytes(bytes[24..32].try_into().unwrap()) as i32;
    let obj = rect_allocate_internal(klass);
    let data = get_rect_mut(obj);
    data.x = x;
    data.y = y;
    data.width = w;
    data.height = h;
    obj
}

pub fn new_rect(x: i32, y: i32, width: i32, height: i32) -> VALUE {
    unsafe {
        let klass = *RECT_CLASS.get().expect("Rect not initialised");
        let value = rect_allocate_internal(klass);
        let rect = get_rect_mut(value);
        rect.x = x;
        rect.y = y;
        rect.width = width;
        rect.height = height;
        value
    }
}

pub fn rect_data(value: VALUE) -> Option<RectData> {
    unsafe { get_typed_data::<RectValue>(value, RECT_TYPE.as_rb_type()) }.map(|rect| RectData {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    })
}
