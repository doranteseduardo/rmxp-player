use super::{
    bitmap::{bitmap_handle, is_bitmap},
    color::{clone_color, get_color_data, is_color, new_color},
    common::{
        bool_to_value, define_method, float_to_value, get_typed_data, install_allocator,
        int_to_value, wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
    rect,
    tone::{clone_tone, is_tone, new_tone, tone_data},
    viewport::{is_viewport, viewport_handle},
};
use crate::native::{self, value_to_bool, value_to_f32, value_to_i32, RectData};
use anyhow::Result;
use once_cell::sync::OnceCell;
use rb_sys::{bindings::rb_gc_mark, VALUE};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_int,
    slice,
};
use tracing::warn;

const SPRITE_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Sprite\0") };
const SPRITE_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Sprite\0") };

static SPRITE_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(SPRITE_STRUCT_NAME)
        .mark(sprite_mark)
        .free(sprite_free)
});
static SPRITE_CLASS: OnceCell<VALUE> = OnceCell::new();

#[derive(Clone)]
struct SpriteValue {
    handle: u32,
    disposed: bool,
    viewport: VALUE,
    bitmap: VALUE,
    src_rect: VALUE,
    color: VALUE,
    tone: VALUE,
    x: f32,
    y: f32,
    z: i32,
    ox: f32,
    oy: f32,
    zoom_x: f32,
    zoom_y: f32,
    angle: f32,
    mirror: bool,
    bush_depth: i32,
    bush_opacity: i32,
    opacity: i32,
    blend_type: i32,
    visible: bool,
}

impl Default for SpriteValue {
    fn default() -> Self {
        Self {
            handle: 0,
            disposed: true,
            viewport: rb_sys::Qnil as VALUE,
            bitmap: rb_sys::Qnil as VALUE,
            src_rect: rb_sys::Qnil as VALUE,
            color: rb_sys::Qnil as VALUE,
            tone: rb_sys::Qnil as VALUE,
            x: 0.0,
            y: 0.0,
            z: 0,
            ox: 0.0,
            oy: 0.0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            angle: 0.0,
            mirror: false,
            bush_depth: 0,
            bush_opacity: 128,
            opacity: 255,
            blend_type: 0,
            visible: true,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(SPRITE_CLASS_NAME, None);
        let _ = SPRITE_CLASS.set(klass);
        install_allocator(klass, Some(sprite_allocate));
        define_method(klass, cstr(b"initialize\0"), sprite_initialize, -1);
        define_method(klass, cstr(b"dispose\0"), sprite_dispose, 0);
        define_method(klass, cstr(b"disposed?\0"), sprite_disposed_q, 0);
        define_method(klass, cstr(b"viewport\0"), sprite_get_viewport, 0);
        define_method(klass, cstr(b"viewport=\0"), sprite_set_viewport, -1);
        define_method(klass, cstr(b"bitmap\0"), sprite_get_bitmap, 0);
        define_method(klass, cstr(b"bitmap=\0"), sprite_set_bitmap, -1);
        define_method(klass, cstr(b"x\0"), sprite_get_x, 0);
        define_method(klass, cstr(b"x=\0"), sprite_set_x, -1);
        define_method(klass, cstr(b"y\0"), sprite_get_y, 0);
        define_method(klass, cstr(b"y=\0"), sprite_set_y, -1);
        define_method(klass, cstr(b"z\0"), sprite_get_z, 0);
        define_method(klass, cstr(b"z=\0"), sprite_set_z, -1);
        define_method(klass, cstr(b"ox\0"), sprite_get_ox, 0);
        define_method(klass, cstr(b"ox=\0"), sprite_set_ox, -1);
        define_method(klass, cstr(b"oy\0"), sprite_get_oy, 0);
        define_method(klass, cstr(b"oy=\0"), sprite_set_oy, -1);
        define_method(klass, cstr(b"width\0"), sprite_get_width, 0);
        define_method(klass, cstr(b"height\0"), sprite_get_height, 0);
        define_method(klass, cstr(b"zoom_x\0"), sprite_get_zoom_x, 0);
        define_method(klass, cstr(b"zoom_x=\0"), sprite_set_zoom_x, -1);
        define_method(klass, cstr(b"zoom_y\0"), sprite_get_zoom_y, 0);
        define_method(klass, cstr(b"zoom_y=\0"), sprite_set_zoom_y, -1);
        define_method(klass, cstr(b"angle\0"), sprite_get_angle, 0);
        define_method(klass, cstr(b"angle=\0"), sprite_set_angle, -1);
        define_method(klass, cstr(b"mirror\0"), sprite_get_mirror, 0);
        define_method(klass, cstr(b"mirror=\0"), sprite_set_mirror, -1);
        define_method(klass, cstr(b"bush_depth\0"), sprite_get_bush_depth, 0);
        define_method(klass, cstr(b"bush_depth=\0"), sprite_set_bush_depth, -1);
        define_method(klass, cstr(b"bush_opacity\0"), sprite_get_bush_opacity, 0);
        define_method(klass, cstr(b"bush_opacity=\0"), sprite_set_bush_opacity, -1);
        define_method(klass, cstr(b"opacity\0"), sprite_get_opacity, 0);
        define_method(klass, cstr(b"opacity=\0"), sprite_set_opacity, -1);
        define_method(klass, cstr(b"blend_type\0"), sprite_get_blend_type, 0);
        define_method(klass, cstr(b"blend_type=\0"), sprite_set_blend_type, -1);
        define_method(klass, cstr(b"visible\0"), sprite_get_visible, 0);
        define_method(klass, cstr(b"visible=\0"), sprite_set_visible, -1);
        define_method(klass, cstr(b"src_rect\0"), sprite_get_src_rect, 0);
        define_method(klass, cstr(b"src_rect=\0"), sprite_set_src_rect, -1);
        define_method(klass, cstr(b"color\0"), sprite_get_color, 0);
        define_method(klass, cstr(b"color=\0"), sprite_set_color, -1);
        define_method(klass, cstr(b"tone\0"), sprite_get_tone, 0);
        define_method(klass, cstr(b"tone=\0"), sprite_set_tone, -1);
        define_method(klass, cstr(b"flash\0"), sprite_flash, -1);
        define_method(klass, cstr(b"update\0"), sprite_update, 0);
        define_method(klass, cstr(b"native_id\0"), sprite_native_id, 0);
    }
    Ok(())
}

unsafe extern "C" fn sprite_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, SpriteValue::default(), SPRITE_TYPE.as_rb_type())
}

unsafe extern "C" fn sprite_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = &*(ptr as *mut SpriteValue);
    if value.viewport != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.viewport);
    }
    if value.bitmap != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.bitmap);
    }
    if value.src_rect != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.src_rect);
    }
    if value.color != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.color);
    }
    if value.tone != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.tone);
    }
}

unsafe extern "C" fn sprite_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = Box::<SpriteValue>::from_raw(ptr as *mut SpriteValue);
    if !value.disposed && value.handle != 0 {
        native::sprite::dispose(value.handle);
    }
}

fn get_sprite(value: VALUE) -> Option<&'static mut SpriteValue> {
    unsafe { get_typed_data(value, SPRITE_TYPE.as_rb_type()) }
}

macro_rules! sprite_or_nil {
    ($val:expr) => {
        match get_sprite($val) {
            Some(s) => s,
            None => return rb_sys::Qnil as VALUE,
        }
    };
}

unsafe extern "C" fn sprite_initialize(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = slice_from(argc, argv);
    let viewport_value = args.get(0).copied().unwrap_or(rb_sys::Qnil as VALUE);
    let viewport_handle = viewport_handle(viewport_value);
    let handle = native::sprite::create(viewport_handle);
    let sprite = sprite_or_nil!(self_value);
    sprite.handle = handle;
    sprite.disposed = false;
    sprite.viewport = viewport_value;
    sprite.bitmap = rb_sys::Qnil as VALUE;
    sprite.src_rect = rect::new_rect(0, 0, 0, 0);
    sprite.color = new_color(0.0, 0.0, 0.0, 0.0);
    sprite.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    sprite.x = 0.0;
    sprite.y = 0.0;
    sprite.z = 0;
    sprite.ox = 0.0;
    sprite.oy = 0.0;
    sprite.zoom_x = 1.0;
    sprite.zoom_y = 1.0;
    sprite.angle = 0.0;
    sprite.mirror = false;
    sprite.bush_depth = 0;
    sprite.bush_opacity = 128;
    sprite.opacity = 255;
    sprite.blend_type = 0;
    sprite.visible = true;
    apply_all(sprite);
    self_value
}

unsafe extern "C" fn sprite_dispose(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let sprite = sprite_or_nil!(self_value);
    if !sprite.disposed && sprite.handle != 0 {
        native::sprite::dispose(sprite.handle);
        sprite.disposed = true;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_disposed_q(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    match get_sprite(self_value) { Some(s) => bool_to_value(s.disposed), None => rb_sys::Qfalse as VALUE }
}

macro_rules! sprite_float_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            match get_sprite(self_value) {
                Some(s) => float_to_value(s.$field as f64),
                None => rb_sys::Qnil as VALUE,
            }
        }
    };
}

macro_rules! sprite_int_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            match get_sprite(self_value) {
                Some(s) => int_to_value(s.$field as i64),
                None => rb_sys::Qnil as VALUE,
            }
        }
    };
}

macro_rules! sprite_bool_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            match get_sprite(self_value) {
                Some(s) => bool_to_value(s.$field),
                None => rb_sys::Qfalse as VALUE,
            }
        }
    };
}

macro_rules! sprite_float_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_f32(*argv);
            let sprite = sprite_or_nil!(self_value);
            sprite.$field = value;
            native::sprite::$setter(sprite.handle, value);
            *argv
        }
    };
}

macro_rules! sprite_int_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_i32(*argv);
            let sprite = sprite_or_nil!(self_value);
            sprite.$field = value;
            native::sprite::$setter(sprite.handle, value);
            *argv
        }
    };
}

macro_rules! sprite_bool_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_bool(*argv);
            let sprite = sprite_or_nil!(self_value);
            sprite.$field = value;
            native::sprite::$setter(sprite.handle, value);
            *argv
        }
    };
}

sprite_float_getter!(sprite_get_x, x);
sprite_float_getter!(sprite_get_y, y);
sprite_int_getter!(sprite_get_z, z);
sprite_float_getter!(sprite_get_ox, ox);
sprite_float_getter!(sprite_get_oy, oy);
sprite_float_getter!(sprite_get_zoom_x, zoom_x);
sprite_float_getter!(sprite_get_zoom_y, zoom_y);
sprite_float_getter!(sprite_get_angle, angle);
sprite_bool_getter!(sprite_get_mirror, mirror);
sprite_int_getter!(sprite_get_bush_depth, bush_depth);
sprite_int_getter!(sprite_get_bush_opacity, bush_opacity);
sprite_int_getter!(sprite_get_opacity, opacity);
sprite_int_getter!(sprite_get_blend_type, blend_type);
sprite_bool_getter!(sprite_get_visible, visible);

sprite_float_setter!(sprite_set_x, x, set_x);
sprite_float_setter!(sprite_set_y, y, set_y);
sprite_int_setter!(sprite_set_z, z, set_z);
sprite_float_setter!(sprite_set_ox, ox, set_ox);
sprite_float_setter!(sprite_set_oy, oy, set_oy);
sprite_float_setter!(sprite_set_zoom_x, zoom_x, set_zoom_x);
sprite_float_setter!(sprite_set_zoom_y, zoom_y, set_zoom_y);
sprite_float_setter!(sprite_set_angle, angle, set_angle);
sprite_bool_setter!(sprite_set_mirror, mirror, set_mirror);
sprite_int_setter!(sprite_set_bush_depth, bush_depth, set_bush_depth);
sprite_int_setter!(sprite_set_bush_opacity, bush_opacity, set_bush_opacity);
sprite_int_setter!(sprite_set_opacity, opacity, set_opacity);
sprite_int_setter!(sprite_set_blend_type, blend_type, set_blend_type);
sprite_bool_setter!(sprite_set_visible, visible, set_visible);

unsafe extern "C" fn sprite_get_width(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let (width, _) = sprite_dimensions(self_value);
    int_to_value(width as i64)
}

unsafe extern "C" fn sprite_get_height(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let (_, height) = sprite_dimensions(self_value);
    int_to_value(height as i64)
}

unsafe extern "C" fn sprite_get_viewport(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    match get_sprite(self_value) { Some(s) => s.viewport, None => rb_sys::Qnil as VALUE }
}

unsafe extern "C" fn sprite_set_viewport(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_viewport(value) {
        warn!(target: "rgss", "Assigned non-Viewport to Sprite#viewport");
        return rb_sys::Qnil as VALUE;
    }
    let handle = viewport_handle(value);
    let sprite = sprite_or_nil!(self_value);
    sprite.viewport = value;
    native::sprite::set_viewport(sprite.handle, handle);
    value
}

unsafe extern "C" fn sprite_get_bitmap(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    match get_sprite(self_value) { Some(s) => s.bitmap, None => rb_sys::Qnil as VALUE }
}

unsafe extern "C" fn sprite_set_bitmap(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let handle = if value == rb_sys::Qnil as VALUE {
        None
    } else if is_bitmap(value) {
        bitmap_handle(value)
    } else {
        warn!(target: "rgss", "Assigned non-Bitmap to Sprite#bitmap");
        None
    };
    let sprite = sprite_or_nil!(self_value);
    sprite.bitmap = if handle.is_some() {
        value
    } else {
        rb_sys::Qnil as VALUE
    };
    native::sprite::set_bitmap(sprite.handle, handle);
    value
}

unsafe extern "C" fn sprite_get_src_rect(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let sprite = sprite_or_nil!(self_value);
    if sprite.src_rect == rb_sys::Qnil as VALUE {
        sprite.src_rect = rect::new_rect(0, 0, 0, 0);
    }
    sprite.src_rect
}

unsafe extern "C" fn sprite_set_src_rect(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    if let Some(data) = rect::rect_data(*argv) {
        let sprite = sprite_or_nil!(self_value);
        sprite.src_rect = rect::new_rect(data.x, data.y, data.width, data.height);
        native::sprite::set_src_rect(sprite.handle, data);
        *argv
    } else {
        rb_sys::Qnil as VALUE
    }
}

unsafe extern "C" fn sprite_get_color(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let sprite = sprite_or_nil!(self_value);
    if sprite.color == rb_sys::Qnil as VALUE {
        sprite.color = new_color(0.0, 0.0, 0.0, 0.0);
    }
    sprite.color
}

unsafe extern "C" fn sprite_set_color(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if !is_color(value) {
        return rb_sys::Qnil as VALUE;
    }
    let sprite = sprite_or_nil!(self_value);
    let color = clone_color(value);
    sprite.color = color;
    native::sprite::set_color(sprite.handle, get_color_data(color));
    value
}

unsafe extern "C" fn sprite_get_tone(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let sprite = sprite_or_nil!(self_value);
    if sprite.tone == rb_sys::Qnil as VALUE {
        sprite.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    }
    sprite.tone
}

unsafe extern "C" fn sprite_set_tone(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if !is_tone(value) {
        return rb_sys::Qnil as VALUE;
    }
    let sprite = sprite_or_nil!(self_value);
    let tone = clone_tone(value);
    sprite.tone = tone;
    native::sprite::set_tone(sprite.handle, tone_data(tone));
    value
}

unsafe extern "C" fn sprite_flash(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = slice_from(argc, argv);
    if args.is_empty() {
        return rb_sys::Qnil as VALUE;
    }
    let sprite = sprite_or_nil!(self_value);
    if sprite.disposed {
        return rb_sys::Qnil as VALUE;
    }
    let duration = if args.len() >= 2 {
        value_to_i32(args[1])
    } else {
        0
    };
    if duration < 1 {
        return rb_sys::Qnil as VALUE;
    }
    let color_value = args[0];
    if color_value == rb_sys::Qnil as VALUE {
        native::sprite::start_flash(sprite.handle, None, duration);
        return rb_sys::Qnil as VALUE;
    }
    if !is_color(color_value) {
        warn!(target: "rgss", "Sprite#flash expected Color or nil");
        return rb_sys::Qnil as VALUE;
    }
    native::sprite::start_flash(sprite.handle, Some(get_color_data(color_value)), duration);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_update(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let sprite = sprite_or_nil!(self_value);
    if sprite.disposed {
        return rb_sys::Qnil as VALUE;
    }
    native::sprite::advance_flash(sprite.handle);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_native_id(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    match get_sprite(self_value) { Some(s) => int_to_value(s.handle as i64), None => rb_sys::Qnil as VALUE }
}

fn apply_all(sprite: &SpriteValue) {
    native::sprite::set_viewport(sprite.handle, viewport_handle(sprite.viewport));
    native::sprite::set_bitmap(sprite.handle, bitmap_handle(sprite.bitmap));
    native::sprite::set_x(sprite.handle, sprite.x);
    native::sprite::set_y(sprite.handle, sprite.y);
    native::sprite::set_z(sprite.handle, sprite.z);
    native::sprite::set_ox(sprite.handle, sprite.ox);
    native::sprite::set_oy(sprite.handle, sprite.oy);
    native::sprite::set_zoom_x(sprite.handle, sprite.zoom_x);
    native::sprite::set_zoom_y(sprite.handle, sprite.zoom_y);
    native::sprite::set_angle(sprite.handle, sprite.angle);
    native::sprite::set_mirror(sprite.handle, sprite.mirror);
    native::sprite::set_bush_depth(sprite.handle, sprite.bush_depth);
    native::sprite::set_bush_opacity(sprite.handle, sprite.bush_opacity);
    native::sprite::set_opacity(sprite.handle, sprite.opacity);
    native::sprite::set_blend_type(sprite.handle, sprite.blend_type);
    native::sprite::set_visible(sprite.handle, sprite.visible);
    let rect_data = rect::rect_data(sprite.src_rect).unwrap_or_else(|| RectData::new(0, 0, 0, 0));
    native::sprite::set_src_rect(sprite.handle, rect_data);
    native::sprite::set_color(sprite.handle, get_color_data(sprite.color));
    native::sprite::set_tone(sprite.handle, tone_data(sprite.tone));
}

fn sprite_dimensions(value: VALUE) -> (i32, i32) {
    let sprite = match get_sprite(value) { Some(s) => s, None => return (0, 0) };
    if sprite.bitmap != rb_sys::Qnil as VALUE {
        if let Some(handle) = bitmap_handle(sprite.bitmap) {
            if let Some((width, height)) = native::bitmap::dimensions(handle) {
                return (width as i32, height as i32);
            }
        }
    }
    let rect = rect::rect_data(sprite.src_rect).unwrap_or_else(|| RectData::new(0, 0, 0, 0));
    (rect.width, rect.height)
}

fn slice_from<'a>(argc: c_int, argv: *const VALUE) -> &'a [VALUE] {
    if argc <= 0 || argv.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(argv, argc as usize) }
    }
}

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}
