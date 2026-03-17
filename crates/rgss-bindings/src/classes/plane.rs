use super::{
    bitmap::{bitmap_handle, is_bitmap},
    color::{clone_color, get_color_data, is_color, new_color},
    common::{
        bool_to_value, define_method, float_to_value, get_typed_data, install_allocator,
        int_to_value, wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
    tone::{clone_tone, is_tone, new_tone, tone_data},
    viewport::{is_viewport, viewport_handle},
};
use crate::native::{self, value_to_bool, value_to_f32, value_to_i32};
use anyhow::Result;
use once_cell::sync::OnceCell;
use rb_sys::{bindings::rb_gc_mark, VALUE};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_int,
    slice,
};
use tracing::warn;

const PLANE_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Plane\0") };
const PLANE_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Plane\0") };

static PLANE_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(PLANE_STRUCT_NAME)
        .mark(plane_mark)
        .free(plane_free)
});
static PLANE_CLASS: OnceCell<VALUE> = OnceCell::new();

#[derive(Clone)]
struct PlaneValue {
    handle: u32,
    disposed: bool,
    viewport: VALUE,
    bitmap: VALUE,
    tone: VALUE,
    color: VALUE,
    z: i32,
    ox: f32,
    oy: f32,
    zoom_x: f32,
    zoom_y: f32,
    opacity: i32,
    blend_type: i32,
    visible: bool,
}

impl Default for PlaneValue {
    fn default() -> Self {
        Self {
            handle: 0,
            disposed: true,
            viewport: rb_sys::Qnil as VALUE,
            bitmap: rb_sys::Qnil as VALUE,
            tone: rb_sys::Qnil as VALUE,
            color: rb_sys::Qnil as VALUE,
            z: 0,
            ox: 0.0,
            oy: 0.0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            opacity: 255,
            blend_type: 0,
            visible: true,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(PLANE_CLASS_NAME, None);
        let _ = PLANE_CLASS.set(klass);
        install_allocator(klass, Some(plane_allocate));
        define_method(klass, cstr(b"initialize\0"), plane_initialize, -1);
        define_method(klass, cstr(b"dispose\0"), plane_dispose, 0);
        define_method(klass, cstr(b"disposed?\0"), plane_disposed_q, 0);
        define_method(klass, cstr(b"viewport\0"), plane_get_viewport, 0);
        define_method(klass, cstr(b"viewport=\0"), plane_set_viewport, -1);
        define_method(klass, cstr(b"bitmap\0"), plane_get_bitmap, 0);
        define_method(klass, cstr(b"bitmap=\0"), plane_set_bitmap, -1);
        define_method(klass, cstr(b"z\0"), plane_get_z, 0);
        define_method(klass, cstr(b"z=\0"), plane_set_z, -1);
        define_method(klass, cstr(b"ox\0"), plane_get_ox, 0);
        define_method(klass, cstr(b"ox=\0"), plane_set_ox, -1);
        define_method(klass, cstr(b"oy\0"), plane_get_oy, 0);
        define_method(klass, cstr(b"oy=\0"), plane_set_oy, -1);
        define_method(klass, cstr(b"zoom_x\0"), plane_get_zoom_x, 0);
        define_method(klass, cstr(b"zoom_x=\0"), plane_set_zoom_x, -1);
        define_method(klass, cstr(b"zoom_y\0"), plane_get_zoom_y, 0);
        define_method(klass, cstr(b"zoom_y=\0"), plane_set_zoom_y, -1);
        define_method(klass, cstr(b"opacity\0"), plane_get_opacity, 0);
        define_method(klass, cstr(b"opacity=\0"), plane_set_opacity, -1);
        define_method(klass, cstr(b"blend_type\0"), plane_get_blend_type, 0);
        define_method(klass, cstr(b"blend_type=\0"), plane_set_blend_type, -1);
        define_method(klass, cstr(b"visible\0"), plane_get_visible, 0);
        define_method(klass, cstr(b"visible=\0"), plane_set_visible, -1);
        define_method(klass, cstr(b"tone\0"), plane_get_tone, 0);
        define_method(klass, cstr(b"tone=\0"), plane_set_tone, -1);
        define_method(klass, cstr(b"color\0"), plane_get_color, 0);
        define_method(klass, cstr(b"color=\0"), plane_set_color, -1);
        define_method(klass, cstr(b"native_id\0"), plane_native_id, 0);
        define_method(klass, cstr(b"update\0"), plane_update, 0);
    }
    Ok(())
}

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}

unsafe extern "C" fn plane_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, PlaneValue::default(), PLANE_TYPE.as_rb_type())
}

unsafe extern "C" fn plane_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let plane = &*(ptr as *mut PlaneValue);
    if plane.viewport != rb_sys::Qnil as VALUE {
        rb_gc_mark(plane.viewport);
    }
    if plane.bitmap != rb_sys::Qnil as VALUE {
        rb_gc_mark(plane.bitmap);
    }
    if plane.tone != rb_sys::Qnil as VALUE {
        rb_gc_mark(plane.tone);
    }
    if plane.color != rb_sys::Qnil as VALUE {
        rb_gc_mark(plane.color);
    }
}

unsafe extern "C" fn plane_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = Box::<PlaneValue>::from_raw(ptr as *mut PlaneValue);
    if !value.disposed && value.handle != 0 {
        native::plane::dispose(value.handle);
    }
}

fn get_plane(value: VALUE) -> &'static mut PlaneValue {
    unsafe { get_typed_data(value, PLANE_TYPE.as_rb_type()) }.expect("Plane missing native data")
}

unsafe extern "C" fn plane_initialize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let viewport_value = args.get(0).copied().unwrap_or(rb_sys::Qnil as VALUE);
    if viewport_value != rb_sys::Qnil as VALUE && !is_viewport(viewport_value) {
        warn!(target: "rgss", "Plane#initialize received non-Viewport");
    }
    let handle = native::plane::create(viewport_handle(viewport_value));
    let plane = get_plane(self_value);
    plane.handle = handle;
    plane.disposed = false;
    plane.viewport = viewport_value;
    plane.bitmap = rb_sys::Qnil as VALUE;
    plane.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    plane.color = new_color(0.0, 0.0, 0.0, 0.0);
    plane.z = 0;
    plane.ox = 0.0;
    plane.oy = 0.0;
    plane.zoom_x = 1.0;
    plane.zoom_y = 1.0;
    plane.opacity = 255;
    plane.blend_type = 0;
    plane.visible = true;
    apply_all(plane);
    self_value
}

unsafe extern "C" fn plane_dispose(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let plane = get_plane(self_value);
    if !plane.disposed && plane.handle != 0 {
        native::plane::dispose(plane.handle);
        plane.disposed = true;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn plane_disposed_q(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    bool_to_value(get_plane(self_value).disposed)
}

macro_rules! plane_float_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            float_to_value(get_plane(self_value).$field as f64)
        }
    };
}

macro_rules! plane_int_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            int_to_value(get_plane(self_value).$field as i64)
        }
    };
}

macro_rules! plane_bool_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            bool_to_value(get_plane(self_value).$field)
        }
    };
}

macro_rules! plane_float_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_f32(*argv);
            let plane = get_plane(self_value);
            plane.$field = value;
            native::plane::$setter(plane.handle, value);
            *argv
        }
    };
}

macro_rules! plane_int_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_i32(*argv);
            let plane = get_plane(self_value);
            plane.$field = value;
            native::plane::$setter(plane.handle, value);
            *argv
        }
    };
}

macro_rules! plane_bool_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_bool(*argv);
            let plane = get_plane(self_value);
            plane.$field = value;
            native::plane::$setter(plane.handle, value);
            *argv
        }
    };
}

plane_int_getter!(plane_get_z, z);
plane_float_getter!(plane_get_ox, ox);
plane_float_getter!(plane_get_oy, oy);
plane_float_getter!(plane_get_zoom_x, zoom_x);
plane_float_getter!(plane_get_zoom_y, zoom_y);
plane_int_getter!(plane_get_opacity, opacity);
plane_int_getter!(plane_get_blend_type, blend_type);
plane_bool_getter!(plane_get_visible, visible);

plane_int_setter!(plane_set_z, z, set_z);
plane_float_setter!(plane_set_ox, ox, set_ox);
plane_float_setter!(plane_set_oy, oy, set_oy);
plane_float_setter!(plane_set_zoom_x, zoom_x, set_zoom_x);
plane_float_setter!(plane_set_zoom_y, zoom_y, set_zoom_y);
plane_int_setter!(plane_set_opacity, opacity, set_opacity);
plane_int_setter!(plane_set_blend_type, blend_type, set_blend_type);
plane_bool_setter!(plane_set_visible, visible, set_visible);

unsafe extern "C" fn plane_get_viewport(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_plane(self_value).viewport
}

unsafe extern "C" fn plane_set_viewport(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_viewport(value) {
        warn!(target: "rgss", "Plane#viewport= received non-Viewport");
        return rb_sys::Qnil as VALUE;
    }
    let handle = viewport_handle(value);
    let plane = get_plane(self_value);
    plane.viewport = value;
    native::plane::set_viewport(plane.handle, handle);
    value
}

unsafe extern "C" fn plane_get_bitmap(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_plane(self_value).bitmap
}

unsafe extern "C" fn plane_set_bitmap(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_bitmap(value) {
        warn!(target: "rgss", "Plane#bitmap= received non-Bitmap");
        return rb_sys::Qnil as VALUE;
    }
    let handle = bitmap_handle(value);
    let plane = get_plane(self_value);
    plane.bitmap = if handle.is_some() {
        value
    } else {
        rb_sys::Qnil as VALUE
    };
    native::plane::set_bitmap(plane.handle, handle);
    value
}

unsafe extern "C" fn plane_get_tone(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let plane = get_plane(self_value);
    if plane.tone == rb_sys::Qnil as VALUE {
        plane.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    }
    plane.tone
}

unsafe extern "C" fn plane_set_tone(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if !is_tone(value) {
        return rb_sys::Qnil as VALUE;
    }
    let plane = get_plane(self_value);
    let tone = clone_tone(value);
    plane.tone = tone;
    native::plane::set_tone(plane.handle, tone_data(tone));
    value
}

unsafe extern "C" fn plane_get_color(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let plane = get_plane(self_value);
    if plane.color == rb_sys::Qnil as VALUE {
        plane.color = new_color(0.0, 0.0, 0.0, 0.0);
    }
    plane.color
}

unsafe extern "C" fn plane_set_color(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if !is_color(value) {
        return rb_sys::Qnil as VALUE;
    }
    let plane = get_plane(self_value);
    let color = clone_color(value);
    plane.color = color;
    native::plane::set_color(plane.handle, get_color_data(color));
    value
}

unsafe extern "C" fn plane_native_id(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    int_to_value(get_plane(self_value).handle as i64)
}

unsafe extern "C" fn plane_update(_argc: c_int, _argv: *const VALUE, _self_value: VALUE) -> VALUE {
    rb_sys::Qnil as VALUE
}

fn apply_all(plane: &PlaneValue) {
    native::plane::set_viewport(plane.handle, viewport_handle(plane.viewport));
    native::plane::set_bitmap(plane.handle, bitmap_handle(plane.bitmap));
    native::plane::set_z(plane.handle, plane.z);
    native::plane::set_ox(plane.handle, plane.ox);
    native::plane::set_oy(plane.handle, plane.oy);
    native::plane::set_zoom_x(plane.handle, plane.zoom_x);
    native::plane::set_zoom_y(plane.handle, plane.zoom_y);
    native::plane::set_opacity(plane.handle, plane.opacity);
    native::plane::set_blend_type(plane.handle, plane.blend_type);
    native::plane::set_visible(plane.handle, plane.visible);
    native::plane::set_tone(plane.handle, tone_data(plane.tone));
    native::plane::set_color(plane.handle, get_color_data(plane.color));
}
