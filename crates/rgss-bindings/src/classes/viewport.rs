use super::{
    color::{clone_color, get_color_data, is_color, new_color},
    common::{
        bool_to_value, define_method, get_typed_data, install_allocator, int_to_value,
        wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
    rect,
    tone::{clone_tone, is_tone, new_tone, tone_data},
};
use crate::native::{self, value_to_bool, value_to_i32, ColorData, RectData, ToneData};
use anyhow::Result;
use once_cell::sync::OnceCell;
use rb_sys::{bindings::rb_gc_mark, VALUE};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_int,
    slice,
};

const VIEWPORT_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Viewport\0") };
const VIEWPORT_STRUCT_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Viewport\0") };

static VIEWPORT_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(VIEWPORT_STRUCT_NAME)
        .mark(viewport_mark)
        .free(viewport_free)
});
static VIEWPORT_CLASS: OnceCell<VALUE> = OnceCell::new();

#[derive(Clone)]
struct ViewportValue {
    handle: u32,
    disposed: bool,
    rect: VALUE,
    visible: bool,
    z: i32,
    ox: i32,
    oy: i32,
    color: VALUE,
    tone: VALUE,
}

impl Default for ViewportValue {
    fn default() -> Self {
        Self {
            handle: 0,
            disposed: true,
            rect: rb_sys::Qnil as VALUE,
            visible: true,
            z: 0,
            ox: 0,
            oy: 0,
            color: rb_sys::Qnil as VALUE,
            tone: rb_sys::Qnil as VALUE,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(VIEWPORT_CLASS_NAME, None);
        let _ = VIEWPORT_CLASS.set(klass);
        install_allocator(klass, Some(viewport_allocate));
        define_method(klass, cstr(b"initialize\0"), viewport_initialize, -1);
        define_method(klass, cstr(b"update\0"), viewport_update, -1);
        define_method(klass, cstr(b"disposed?\0"), viewport_disposed_q, -1);
        define_method(klass, cstr(b"dispose\0"), viewport_dispose, -1);
        define_method(klass, cstr(b"rect\0"), viewport_get_rect, -1);
        define_method(klass, cstr(b"rect=\0"), viewport_set_rect, -1);
        define_method(klass, cstr(b"visible\0"), viewport_get_visible, -1);
        define_method(klass, cstr(b"visible=\0"), viewport_set_visible, -1);
        define_method(klass, cstr(b"z\0"), viewport_get_z, -1);
        define_method(klass, cstr(b"z=\0"), viewport_set_z, -1);
        define_method(klass, cstr(b"ox\0"), viewport_get_ox, -1);
        define_method(klass, cstr(b"ox=\0"), viewport_set_ox, -1);
        define_method(klass, cstr(b"oy\0"), viewport_get_oy, -1);
        define_method(klass, cstr(b"oy=\0"), viewport_set_oy, -1);
        define_method(klass, cstr(b"color\0"), viewport_get_color, -1);
        define_method(klass, cstr(b"color=\0"), viewport_set_color, -1);
        define_method(klass, cstr(b"tone\0"), viewport_get_tone, -1);
        define_method(klass, cstr(b"tone=\0"), viewport_set_tone, -1);
        define_method(klass, cstr(b"native_id\0"), viewport_native_id, -1);
    }
    Ok(())
}

unsafe extern "C" fn viewport_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, ViewportValue::default(), VIEWPORT_TYPE.as_rb_type())
}

unsafe extern "C" fn viewport_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = &*(ptr as *mut ViewportValue);
    if value.rect != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.rect);
    }
    if value.color != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.color);
    }
    if value.tone != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.tone);
    }
}

unsafe extern "C" fn viewport_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = Box::<ViewportValue>::from_raw(ptr as *mut ViewportValue);
    if !value.disposed && value.handle != 0 {
        native::viewport::dispose(value.handle);
    }
}

fn get_viewport(value: VALUE) -> &'static mut ViewportValue {
    unsafe { get_typed_data(value, VIEWPORT_TYPE.as_rb_type()) }.expect("Viewport missing data")
}

unsafe extern "C" fn viewport_initialize(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = slice_from(argc, argv);
    let (rect_value, rect_data) = parse_rect_args(args);
    let handle = native::viewport::create(rect_data);
    let viewport = get_viewport(self_value);
    viewport.handle = handle;
    viewport.disposed = false;
    viewport.rect = rect_value;
    viewport.visible = true;
    viewport.z = 0;
    viewport.ox = 0;
    viewport.oy = 0;
    viewport.color = new_color(0.0, 0.0, 0.0, 0.0);
    viewport.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    native::viewport::set_rect(handle, rect_data);
    native::viewport::set_visible(handle, true);
    native::viewport::set_z(handle, 0);
    native::viewport::set_ox(handle, 0);
    native::viewport::set_oy(handle, 0);
    native::viewport::set_color(handle, ColorData::default());
    native::viewport::set_tone(handle, ToneData::default());
    self_value
}

unsafe extern "C" fn viewport_update(
    _argc: c_int,
    _argv: *const VALUE,
    _self_value: VALUE,
) -> VALUE {
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_disposed_q(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    bool_to_value(get_viewport(self_value).disposed)
}

unsafe extern "C" fn viewport_dispose(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let viewport = get_viewport(self_value);
    if !viewport.disposed && viewport.handle != 0 {
        native::viewport::dispose(viewport.handle);
        viewport.disposed = true;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_get_rect(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let viewport = get_viewport(self_value);
    if viewport.rect == rb_sys::Qnil as VALUE {
        viewport.rect = rect::new_rect(0, 0, 0, 0);
    }
    viewport.rect
}

unsafe extern "C" fn viewport_set_rect(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let (rect_value, rect_data) = rect_from_value(value);
    let viewport = get_viewport(self_value);
    viewport.rect = rect_value;
    native::viewport::set_rect(viewport.handle, rect_data);
    value
}

unsafe extern "C" fn viewport_get_visible(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    bool_to_value(get_viewport(self_value).visible)
}

unsafe extern "C" fn viewport_set_visible(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = value_to_bool(*argv);
    let viewport = get_viewport(self_value);
    viewport.visible = value;
    native::viewport::set_visible(viewport.handle, value);
    *argv
}

unsafe extern "C" fn viewport_get_z(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    int_to_value(get_viewport(self_value).z as i64)
}

unsafe extern "C" fn viewport_set_z(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = value_to_i32(*argv);
    let viewport = get_viewport(self_value);
    viewport.z = value;
    native::viewport::set_z(viewport.handle, value);
    *argv
}

unsafe extern "C" fn viewport_get_ox(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    int_to_value(get_viewport(self_value).ox as i64)
}

unsafe extern "C" fn viewport_set_ox(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = value_to_i32(*argv);
    let viewport = get_viewport(self_value);
    viewport.ox = value;
    native::viewport::set_ox(viewport.handle, value);
    *argv
}

unsafe extern "C" fn viewport_get_oy(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    int_to_value(get_viewport(self_value).oy as i64)
}

unsafe extern "C" fn viewport_set_oy(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = value_to_i32(*argv);
    let viewport = get_viewport(self_value);
    viewport.oy = value;
    native::viewport::set_oy(viewport.handle, value);
    *argv
}

unsafe extern "C" fn viewport_get_color(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let viewport = get_viewport(self_value);
    if viewport.color == rb_sys::Qnil as VALUE {
        viewport.color = new_color(0.0, 0.0, 0.0, 0.0);
    }
    viewport.color
}

unsafe extern "C" fn viewport_set_color(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let color_value = if is_color(value) {
        clone_color(value)
    } else {
        new_color(0.0, 0.0, 0.0, 0.0)
    };
    let viewport = get_viewport(self_value);
    viewport.color = color_value;
    native::viewport::set_color(viewport.handle, get_color_data(color_value));
    value
}

unsafe extern "C" fn viewport_get_tone(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let viewport = get_viewport(self_value);
    if viewport.tone == rb_sys::Qnil as VALUE {
        viewport.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    }
    viewport.tone
}

unsafe extern "C" fn viewport_set_tone(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let tone_value = if is_tone(value) {
        clone_tone(value)
    } else {
        new_tone(0.0, 0.0, 0.0, 0.0)
    };
    let viewport = get_viewport(self_value);
    viewport.tone = tone_value;
    native::viewport::set_tone(viewport.handle, tone_data(tone_value));
    value
}

unsafe extern "C" fn viewport_native_id(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    int_to_value(get_viewport(self_value).handle as i64)
}

fn slice_from<'a>(argc: c_int, argv: *const VALUE) -> &'a [VALUE] {
    if argc <= 0 || argv.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(argv, argc as usize) }
    }
}

fn parse_rect_args(args: &[VALUE]) -> (VALUE, RectData) {
    if let Some(first) = args.first() {
        if let Some(data) = rect::rect_data(*first) {
            let rect_value = rect::new_rect(data.x, data.y, data.width, data.height);
            return (rect_value, data);
        }
    }
    let x = args.get(0).map(|v| value_to_i32(*v)).unwrap_or(0);
    let y = args.get(1).map(|v| value_to_i32(*v)).unwrap_or(0);
    let width = args.get(2).map(|v| value_to_i32(*v)).unwrap_or(0);
    let height = args.get(3).map(|v| value_to_i32(*v)).unwrap_or(0);
    let data = RectData::new(x, y, width, height);
    let rect_value = rect::new_rect(data.x, data.y, data.width, data.height);
    (rect_value, data)
}

fn rect_from_value(value: VALUE) -> (VALUE, RectData) {
    if let Some(data) = rect::rect_data(value) {
        let rect_value = rect::new_rect(data.x, data.y, data.width, data.height);
        (rect_value, data)
    } else {
        let data = RectData::new(0, 0, 0, 0);
        let rect_value = rect::new_rect(0, 0, 0, 0);
        (rect_value, data)
    }
}

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}

pub fn is_viewport(value: VALUE) -> bool {
    unsafe { get_typed_data::<ViewportValue>(value, VIEWPORT_TYPE.as_rb_type()).is_some() }
}

pub fn viewport_handle(value: VALUE) -> Option<u32> {
    unsafe { get_typed_data::<ViewportValue>(value, VIEWPORT_TYPE.as_rb_type()) }.and_then(
        |viewport| {
            if viewport.disposed {
                None
            } else {
                Some(viewport.handle)
            }
        },
    )
}
