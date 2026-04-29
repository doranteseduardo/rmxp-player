use super::{
    color::{clone_color, get_color_data, is_color, new_color},
    common::{
        bool_to_value, define_method, define_singleton_method, get_typed_data, install_allocator,
        int_to_value, to_c_long, utf8_str, wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
};
use crate::native::ColorData;
use crate::native::{value_to_bool, value_to_i32};
use anyhow::Result;
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    bindings::{
        rb_Array, rb_String, rb_ary_new_capa, rb_ary_push, rb_class_new_instance, rb_gc_mark,
        rb_obj_class, rb_string_value_cstr,
    },
    macros::RARRAY_LEN,
    VALUE,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::{c_int, c_long},
    ptr, slice,
    sync::Mutex,
};

const FONT_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Font\0") };
const FONT_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Font\0") };

static FONT_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(FONT_STRUCT_NAME)
        .mark(font_mark)
        .free(font_free)
});
static FONT_CLASS: OnceCell<VALUE> = OnceCell::new();

static DEFAULTS: Lazy<Mutex<FontDefaults>> = Lazy::new(|| Mutex::new(FontDefaults::default()));

#[derive(Clone)]
struct FontValue {
    name: Vec<String>,
    size: i32,
    bold: bool,
    italic: bool,
    shadow: bool,
    color: VALUE,
}

#[derive(Clone)]
struct FontDefaults {
    name: Vec<String>,
    size: i32,
    bold: bool,
    italic: bool,
    shadow: bool,
    color: (f32, f32, f32, f32),
}

impl Default for FontDefaults {
    fn default() -> Self {
        Self {
            name: vec!["Arial".to_string()],
            size: 24,
            bold: false,
            italic: false,
            shadow: false,
            color: (255.0, 255.0, 255.0, 255.0),
        }
    }
}

impl Default for FontValue {
    fn default() -> Self {
        let defaults = DEFAULTS.lock().unwrap().clone();
        Self {
            name: defaults.name.clone(),
            size: defaults.size,
            bold: defaults.bold,
            italic: defaults.italic,
            shadow: defaults.shadow,
            color: new_color(
                defaults.color.0,
                defaults.color.1,
                defaults.color.2,
                defaults.color.3,
            ),
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(FONT_CLASS_NAME, None);
        let _ = FONT_CLASS.set(klass);
        install_allocator(klass, Some(font_allocate));
        define_method(klass, cstr(b"initialize\0"), font_initialize, -1);
        define_method(klass, cstr(b"name\0"), font_get_name, -1);
        define_method(klass, cstr(b"name=\0"), font_set_name, -1);
        define_method(klass, cstr(b"size\0"), font_get_size, -1);
        define_method(klass, cstr(b"size=\0"), font_set_size, -1);
        define_method(klass, cstr(b"bold\0"), font_get_bold, -1);
        define_method(klass, cstr(b"bold=\0"), font_set_bold, -1);
        define_method(klass, cstr(b"italic\0"), font_get_italic, -1);
        define_method(klass, cstr(b"italic=\0"), font_set_italic, -1);
        define_method(klass, cstr(b"shadow\0"), font_get_shadow, -1);
        define_method(klass, cstr(b"shadow=\0"), font_set_shadow, -1);
        define_method(klass, cstr(b"color\0"), font_get_color, -1);
        define_method(klass, cstr(b"color=\0"), font_set_color, -1);

        define_singleton_method(klass, cstr(b"default_name\0"), font_default_name, -1);
        define_singleton_method(klass, cstr(b"default_name=\0"), font_set_default_name, -1);
        define_singleton_method(klass, cstr(b"default_size\0"), font_default_size, -1);
        define_singleton_method(klass, cstr(b"default_size=\0"), font_set_default_size, -1);
        define_singleton_method(klass, cstr(b"default_bold\0"), font_default_bold, -1);
        define_singleton_method(klass, cstr(b"default_bold=\0"), font_set_default_bold, -1);
        define_singleton_method(klass, cstr(b"default_italic\0"), font_default_italic, -1);
        define_singleton_method(
            klass,
            cstr(b"default_italic=\0"),
            font_set_default_italic,
            1,
        );
        define_singleton_method(klass, cstr(b"default_shadow\0"), font_default_shadow, -1);
        define_singleton_method(
            klass,
            cstr(b"default_shadow=\0"),
            font_set_default_shadow,
            1,
        );
        define_singleton_method(klass, cstr(b"default_color\0"), font_default_color, -1);
        define_singleton_method(klass, cstr(b"default_color=\0"), font_set_default_color, -1);
        define_singleton_method(klass, cstr(b"exist?\0"), font_exist_q, -1);
    }
    Ok(())
}

unsafe extern "C" fn font_exist_q(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    rb_sys::Qtrue as VALUE
}

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}

unsafe extern "C" fn font_allocate(klass: VALUE) -> VALUE {
    font_allocate_internal(klass)
}

unsafe fn font_allocate_internal(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, FontValue::default(), FONT_TYPE.as_rb_type())
}

unsafe extern "C" fn font_free(ptr: *mut c_void) {
    drop(Box::<FontValue>::from_raw(ptr as *mut FontValue));
}

unsafe extern "C" fn font_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let font = &*(ptr as *mut FontValue);
    rb_gc_mark(font.color);
}

fn get_font_mut(value: VALUE) -> &'static mut FontValue {
    unsafe { get_typed_data(value, FONT_TYPE.as_rb_type()) }.expect("Font missing native data")
}

unsafe extern "C" fn font_initialize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let defaults = DEFAULTS.lock().unwrap().clone();
    let font = get_font_mut(self_value);
    font.name = if let Some(value) = args.get(0) {
        normalize_names(*value)
    } else {
        defaults.name.clone()
    };
    font.size = args
        .get(1)
        .map(|v| value_to_i32(*v))
        .unwrap_or(defaults.size);
    font.bold = defaults.bold;
    font.italic = defaults.italic;
    font.shadow = defaults.shadow;
    font.color = new_color(
        defaults.color.0,
        defaults.color.1,
        defaults.color.2,
        defaults.color.3,
    );
    self_value
}

unsafe extern "C" fn font_get_name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    names_to_array(&get_font_mut(self_value).name)
}

unsafe extern "C" fn font_set_name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    get_font_mut(self_value).name = normalize_names(value);
    value
}

unsafe extern "C" fn font_get_size(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    int_to_value(get_font_mut(self_value).size as i64)
}

unsafe extern "C" fn font_set_size(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    get_font_mut(self_value).size = value_to_i32(value);
    value
}

macro_rules! bool_accessor {
    ($getter:ident, $setter:ident, $field:ident) => {
        unsafe extern "C" fn $getter(
            _argc: c_int,
            _argv: *const VALUE,
            self_value: VALUE,
        ) -> VALUE {
            bool_to_value(get_font_mut(self_value).$field)
        }

        unsafe extern "C" fn $setter(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = *argv;
            get_font_mut(self_value).$field = value_to_bool(value);
            value
        }
    };
}

bool_accessor!(font_get_bold, font_set_bold, bold);
bool_accessor!(font_get_italic, font_set_italic, italic);
bool_accessor!(font_get_shadow, font_set_shadow, shadow);

unsafe extern "C" fn font_get_color(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    get_font_mut(self_value).color
}

unsafe extern "C" fn font_set_color(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let color = if is_color(value) {
        clone_color(value)
    } else {
        let defaults = DEFAULTS.lock().unwrap().clone();
        new_color(
            defaults.color.0,
            defaults.color.1,
            defaults.color.2,
            defaults.color.3,
        )
    };
    get_font_mut(self_value).color = color;
    value
}

unsafe extern "C" fn font_default_name(_argc: c_int, _argv: *const VALUE, _klass: VALUE) -> VALUE {
    let defaults = DEFAULTS.lock().unwrap();
    names_to_array(&defaults.name)
}

unsafe extern "C" fn font_set_default_name(
    _argc: c_int,
    argv: *const VALUE,
    _klass: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let mut defaults = DEFAULTS.lock().unwrap();
    defaults.name = normalize_names(value);
    value
}

unsafe extern "C" fn font_default_size(_argc: c_int, _argv: *const VALUE, _klass: VALUE) -> VALUE {
    int_to_value(DEFAULTS.lock().unwrap().size as i64)
}

unsafe extern "C" fn font_set_default_size(
    _argc: c_int,
    argv: *const VALUE,
    _klass: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    DEFAULTS.lock().unwrap().size = value_to_i32(value);
    value
}

macro_rules! default_bool {
    ($getter:ident, $setter:ident, $field:ident) => {
        unsafe extern "C" fn $getter(_argc: c_int, _argv: *const VALUE, _klass: VALUE) -> VALUE {
            bool_to_value(DEFAULTS.lock().unwrap().$field)
        }

        unsafe extern "C" fn $setter(_argc: c_int, argv: *const VALUE, _klass: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = *argv;
            DEFAULTS.lock().unwrap().$field = value_to_bool(value);
            value
        }
    };
}

default_bool!(font_default_bold, font_set_default_bold, bold);
default_bool!(font_default_italic, font_set_default_italic, italic);
default_bool!(font_default_shadow, font_set_default_shadow, shadow);

unsafe extern "C" fn font_default_color(_argc: c_int, _argv: *const VALUE, _klass: VALUE) -> VALUE {
    let defaults = DEFAULTS.lock().unwrap();
    new_color(
        defaults.color.0,
        defaults.color.1,
        defaults.color.2,
        defaults.color.3,
    )
}

unsafe extern "C" fn font_set_default_color(
    _argc: c_int,
    argv: *const VALUE,
    _klass: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let mut defaults = DEFAULTS.lock().unwrap();
    if is_color(value) {
        let color = get_color_components(value);
        defaults.color = color;
    } else {
        defaults.color = (255.0, 255.0, 255.0, 255.0);
    }
    value
}

unsafe fn normalize_names(value: VALUE) -> Vec<String> {
    let array = rb_Array(value);
    let len = RARRAY_LEN(array) as usize;
    let mut names = Vec::with_capacity(len);
    for index in 0..len {
        let entry = rb_sys::bindings::rb_ary_entry(array, index as c_long);
        names.push(value_to_string(entry));
    }
    if names.is_empty() {
        names.push(String::new());
    }
    names
}

unsafe fn names_to_array(names: &[String]) -> VALUE {
    let ary = rb_ary_new_capa(to_c_long(names.len()));
    for name in names {
        rb_ary_push(ary, utf8_str(name));
    }
    ary
}

unsafe fn value_to_string(value: VALUE) -> String {
    let mut coerced = rb_String(value);
    let ptr = rb_string_value_cstr(&mut coerced);
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}

unsafe fn get_color_components(value: VALUE) -> (f32, f32, f32, f32) {
    let color = get_color_data(value);
    (color.red, color.green, color.blue, color.alpha)
}

pub fn new_font() -> VALUE {
    unsafe {
        let klass = *FONT_CLASS.get().expect("Font not initialised");
        rb_class_new_instance(0, ptr::null(), klass)
    }
}

pub fn clone_font(value: VALUE) -> VALUE {
    unsafe {
        let klass = rb_obj_class(value);
        let new_value = font_allocate_internal(klass);
        let source = get_font_mut(value).clone();
        *get_font_mut(new_value) = source;
        new_value
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct FontSnapshot {
    pub names: Vec<String>,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub shadow: bool,
    pub color: ColorData,
}

pub fn font_snapshot(value: VALUE) -> Option<FontSnapshot> {
    unsafe { get_typed_data::<FontValue>(value, FONT_TYPE.as_rb_type()) }.map(|font| FontSnapshot {
        names: font.name.clone(),
        size: font.size,
        bold: font.bold,
        italic: font.italic,
        shadow: font.shadow,
        color: get_color_data(font.color),
    })
}

pub fn is_font(value: VALUE) -> bool {
    unsafe { get_typed_data::<FontValue>(value, FONT_TYPE.as_rb_type()).is_some() }
}
