use super::{
    bitmap::{bitmap_handle, is_bitmap},
    color::{clone_color, get_color_data, is_color, new_color},
    common::{
        bool_to_value, define_method, get_typed_data, install_allocator, int_to_value,
        wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
    rect,
    tone::{clone_tone, is_tone, new_tone, tone_data},
    viewport::{is_viewport, viewport_handle},
};
use crate::native::{self, value_to_bool, value_to_i32, RectData};
use anyhow::Result;
use once_cell::sync::OnceCell;
use rb_sys::{bindings::rb_gc_mark, VALUE};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_int,
    slice,
};
use tracing::warn;

const WINDOW_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Window\0") };
const WINDOW_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Window\0") };

static WINDOW_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(WINDOW_STRUCT_NAME)
        .mark(window_mark)
        .free(window_free)
});
static WINDOW_CLASS: OnceCell<VALUE> = OnceCell::new();

#[derive(Clone)]
struct WindowValue {
    handle: u32,
    disposed: bool,
    viewport: VALUE,
    windowskin: VALUE,
    contents: VALUE,
    cursor_rect: VALUE,
    tone: VALUE,
    color: VALUE,
    x: i32,
    y: i32,
    z: i32,
    ox: i32,
    oy: i32,
    width: i32,
    height: i32,
    opacity: i32,
    back_opacity: i32,
    contents_opacity: i32,
    openness: i32,
    visible: bool,
    active: bool,
    pause: bool,
}

impl Default for WindowValue {
    fn default() -> Self {
        Self {
            handle: 0,
            disposed: true,
            viewport: rb_sys::Qnil as VALUE,
            windowskin: rb_sys::Qnil as VALUE,
            contents: rb_sys::Qnil as VALUE,
            cursor_rect: rb_sys::Qnil as VALUE,
            tone: rb_sys::Qnil as VALUE,
            color: rb_sys::Qnil as VALUE,
            x: 0,
            y: 0,
            z: 0,
            ox: 0,
            oy: 0,
            width: 32,
            height: 32,
            opacity: 255,
            back_opacity: 255,
            contents_opacity: 255,
            openness: 255,
            visible: true,
            active: true,
            pause: false,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(WINDOW_CLASS_NAME, None);
        let _ = WINDOW_CLASS.set(klass);
        install_allocator(klass, Some(window_allocate));
        define_method(klass, cstr(b"initialize\0"), window_initialize, -1);
        define_method(klass, cstr(b"dispose\0"), window_dispose, -1);
        define_method(klass, cstr(b"disposed?\0"), window_disposed_q, -1);
        define_method(klass, cstr(b"viewport\0"), window_get_viewport, -1);
        define_method(klass, cstr(b"viewport=\0"), window_set_viewport, -1);
        define_method(klass, cstr(b"windowskin\0"), window_get_windowskin, -1);
        define_method(klass, cstr(b"windowskin=\0"), window_set_windowskin, -1);
        define_method(klass, cstr(b"contents\0"), window_get_contents, -1);
        define_method(klass, cstr(b"contents=\0"), window_set_contents, -1);
        define_method(klass, cstr(b"x\0"), window_get_x, -1);
        define_method(klass, cstr(b"x=\0"), window_set_x, -1);
        define_method(klass, cstr(b"y\0"), window_get_y, -1);
        define_method(klass, cstr(b"y=\0"), window_set_y, -1);
        define_method(klass, cstr(b"z\0"), window_get_z, -1);
        define_method(klass, cstr(b"z=\0"), window_set_z, -1);
        define_method(klass, cstr(b"width\0"), window_get_width, -1);
        define_method(klass, cstr(b"width=\0"), window_set_width, -1);
        define_method(klass, cstr(b"height\0"), window_get_height, -1);
        define_method(klass, cstr(b"height=\0"), window_set_height, -1);
        define_method(klass, cstr(b"ox\0"), window_get_ox, -1);
        define_method(klass, cstr(b"ox=\0"), window_set_ox, -1);
        define_method(klass, cstr(b"oy\0"), window_get_oy, -1);
        define_method(klass, cstr(b"oy=\0"), window_set_oy, -1);
        define_method(klass, cstr(b"opacity\0"), window_get_opacity, -1);
        define_method(klass, cstr(b"opacity=\0"), window_set_opacity, -1);
        define_method(klass, cstr(b"back_opacity\0"), window_get_back_opacity, -1);
        define_method(klass, cstr(b"back_opacity=\0"), window_set_back_opacity, -1);
        define_method(klass, cstr(b"contents_opacity\0"), window_get_contents_opacity, -1);
        define_method(klass, cstr(b"contents_opacity=\0"), window_set_contents_opacity, -1);
        define_method(klass, cstr(b"openness\0"), window_get_openness, -1);
        define_method(klass, cstr(b"openness=\0"), window_set_openness, -1);
        define_method(klass, cstr(b"visible\0"), window_get_visible, -1);
        define_method(klass, cstr(b"visible=\0"), window_set_visible, -1);
        define_method(klass, cstr(b"active\0"), window_get_active, -1);
        define_method(klass, cstr(b"active=\0"), window_set_active, -1);
        define_method(klass, cstr(b"pause\0"), window_get_pause, -1);
        define_method(klass, cstr(b"pause=\0"), window_set_pause, -1);
        define_method(klass, cstr(b"tone\0"), window_get_tone, -1);
        define_method(klass, cstr(b"tone=\0"), window_set_tone, -1);
        define_method(klass, cstr(b"color\0"), window_get_color, -1);
        define_method(klass, cstr(b"color=\0"), window_set_color, -1);
        define_method(klass, cstr(b"cursor_rect\0"), window_get_cursor_rect, -1);
        define_method(klass, cstr(b"cursor_rect=\0"), window_set_cursor_rect, -1);
        define_method(klass, cstr(b"open\0"), window_open, -1);
        define_method(klass, cstr(b"close\0"), window_close, -1);
        define_method(klass, cstr(b"update\0"), window_update, -1);
        define_method(klass, cstr(b"native_id\0"), window_native_id, -1);
    }
    Ok(())
}

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}

unsafe extern "C" fn window_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, WindowValue::default(), WINDOW_TYPE.as_rb_type())
}

unsafe extern "C" fn window_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let window = &*(ptr as *mut WindowValue);
    for value in [
        window.viewport,
        window.windowskin,
        window.contents,
        window.cursor_rect,
        window.tone,
        window.color,
    ] {
        if value != rb_sys::Qnil as VALUE {
            rb_gc_mark(value);
        }
    }
}

unsafe extern "C" fn window_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = Box::<WindowValue>::from_raw(ptr as *mut WindowValue);
    if !value.disposed && value.handle != 0 {
        native::window::dispose(value.handle);
    }
}

fn get_window(value: VALUE) -> &'static mut WindowValue {
    unsafe { get_typed_data(value, WINDOW_TYPE.as_rb_type()) }.expect("Window missing native data")
}

fn try_get_window(value: VALUE) -> Option<&'static mut WindowValue> {
    unsafe { get_typed_data(value, WINDOW_TYPE.as_rb_type()) }
}

unsafe extern "C" fn window_initialize(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let x = args.get(0).map(|v| value_to_i32(*v)).unwrap_or(0);
    let y = args.get(1).map(|v| value_to_i32(*v)).unwrap_or(0);
    let width = args.get(2).map(|v| value_to_i32(*v)).unwrap_or(32);
    let height = args.get(3).map(|v| value_to_i32(*v)).unwrap_or(32);
    let viewport_value = args.get(4).copied().unwrap_or(rb_sys::Qnil as VALUE);
    if viewport_value != rb_sys::Qnil as VALUE && !is_viewport(viewport_value) {
        warn!(target: "rgss", "Window#initialize received non-Viewport");
    }
    let handle = native::window::create();
    let window = get_window(self_value);
    window.handle = handle;
    window.disposed = false;
    window.viewport = viewport_value;
    window.windowskin = rb_sys::Qnil as VALUE;
    window.contents = rb_sys::Qnil as VALUE;
    window.cursor_rect = rect::new_rect(0, 0, 0, 0);
    window.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    window.color = new_color(0.0, 0.0, 0.0, 0.0);
    window.x = x;
    window.y = y;
    window.width = width.max(0);
    window.height = height.max(0);
    window.z = 0;
    window.ox = 0;
    window.oy = 0;
    window.opacity = 255;
    window.back_opacity = 255;
    window.contents_opacity = 255;
    window.openness = 255;
    window.visible = true;
    window.active = true;
    window.pause = false;
    apply_all(window);
    self_value
}

unsafe extern "C" fn window_dispose(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let window = get_window(self_value);
    if !window.disposed && window.handle != 0 {
        native::window::dispose(window.handle);
        window.disposed = true;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_disposed_q(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    bool_to_value(try_get_window(self_value).map(|w| w.disposed).unwrap_or(true))
}

macro_rules! window_int_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            int_to_value(get_window(self_value).$field as i64)
        }
    };
}

macro_rules! window_bool_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            bool_to_value(get_window(self_value).$field)
        }
    };
}

macro_rules! window_int_setter {
    ($name:ident, $field:ident, $setter:ident, $clamp:expr) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let mut value = value_to_i32(*argv);
            value = $clamp(value);
            let window = get_window(self_value);
            window.$field = value;
            native::window::$setter(window.handle, value);
            *argv
        }
    };
}

macro_rules! window_bool_setter {
    ($name:ident, $field:ident, $setter:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_bool(*argv);
            let window = get_window(self_value);
            window.$field = value;
            native::window::$setter(window.handle, value);
            *argv
        }
    };
}

window_int_getter!(window_get_x, x);
window_int_getter!(window_get_y, y);
window_int_getter!(window_get_z, z);
window_int_getter!(window_get_width, width);
window_int_getter!(window_get_height, height);
window_int_getter!(window_get_ox, ox);
window_int_getter!(window_get_oy, oy);
window_int_getter!(window_get_opacity, opacity);
window_int_getter!(window_get_back_opacity, back_opacity);
window_int_getter!(window_get_contents_opacity, contents_opacity);
window_int_getter!(window_get_openness, openness);
window_bool_getter!(window_get_visible, visible);
window_bool_getter!(window_get_active, active);
window_bool_getter!(window_get_pause, pause);

fn clamp_dimension(value: i32) -> i32 {
    value.max(0)
}

fn clamp_openness(value: i32) -> i32 {
    value.clamp(0, 255)
}

fn clamp_identity(value: i32) -> i32 {
    value
}

window_int_setter!(window_set_x, x, set_x, clamp_identity);
window_int_setter!(window_set_y, y, set_y, clamp_identity);
window_int_setter!(window_set_z, z, set_z, clamp_identity);
window_int_setter!(window_set_width, width, set_width, clamp_dimension);
window_int_setter!(window_set_height, height, set_height, clamp_dimension);
window_int_setter!(window_set_ox, ox, set_ox, clamp_identity);
window_int_setter!(window_set_oy, oy, set_oy, clamp_identity);
window_int_setter!(window_set_opacity, opacity, set_opacity, clamp_identity);
window_int_setter!(
    window_set_back_opacity,
    back_opacity,
    set_back_opacity,
    clamp_identity
);
window_int_setter!(
    window_set_contents_opacity,
    contents_opacity,
    set_contents_opacity,
    clamp_identity
);
window_int_setter!(window_set_openness, openness, set_openness, clamp_openness);
window_bool_setter!(window_set_visible, visible, set_visible);
window_bool_setter!(window_set_active, active, set_active);
window_bool_setter!(window_set_pause, pause, set_pause);

unsafe extern "C" fn window_get_viewport(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_window(self_value).viewport
}

unsafe extern "C" fn window_set_viewport(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_viewport(value) {
        warn!(target: "rgss", "Window#viewport= received non-Viewport");
        return rb_sys::Qnil as VALUE;
    }
    let handle = viewport_handle(value);
    let window = get_window(self_value);
    window.viewport = value;
    native::window::set_viewport(window.handle, handle);
    value
}

unsafe extern "C" fn window_get_windowskin(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_window(self_value).windowskin
}

unsafe extern "C" fn window_set_windowskin(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_bitmap(value) {
        warn!(target: "rgss", "Window#windowskin= received non-Bitmap");
        return rb_sys::Qnil as VALUE;
    }
    let handle = bitmap_handle(value);
    let window = get_window(self_value);
    window.windowskin = if handle.is_some() {
        value
    } else {
        rb_sys::Qnil as VALUE
    };
    native::window::set_windowskin(window.handle, handle);
    value
}

unsafe extern "C" fn window_get_contents(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_window(self_value).contents
}

unsafe extern "C" fn window_set_contents(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_bitmap(value) {
        warn!(target: "rgss", "Window#contents= received non-Bitmap");
        return rb_sys::Qnil as VALUE;
    }
    let handle = bitmap_handle(value);
    let window = get_window(self_value);
    window.contents = if handle.is_some() {
        value
    } else {
        rb_sys::Qnil as VALUE
    };
    native::window::set_contents(window.handle, handle);
    value
}

unsafe extern "C" fn window_get_tone(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let window = get_window(self_value);
    if window.tone == rb_sys::Qnil as VALUE {
        window.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    }
    window.tone
}

unsafe extern "C" fn window_set_tone(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if !is_tone(value) {
        return rb_sys::Qnil as VALUE;
    }
    let window = get_window(self_value);
    let tone = clone_tone(value);
    window.tone = tone;
    native::window::set_tone(window.handle, tone_data(tone));
    value
}

unsafe extern "C" fn window_get_color(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let window = get_window(self_value);
    if window.color == rb_sys::Qnil as VALUE {
        window.color = new_color(0.0, 0.0, 0.0, 0.0);
    }
    window.color
}

unsafe extern "C" fn window_set_color(
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
    let window = get_window(self_value);
    let color = clone_color(value);
    window.color = color;
    native::window::set_color(window.handle, get_color_data(color));
    value
}

unsafe extern "C" fn window_get_cursor_rect(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let window = get_window(self_value);
    if window.cursor_rect == rb_sys::Qnil as VALUE {
        window.cursor_rect = rect::new_rect(0, 0, 0, 0);
    }
    window.cursor_rect
}

unsafe extern "C" fn window_set_cursor_rect(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    if let Some(data) = rect::rect_data(*argv) {
        let window = get_window(self_value);
        window.cursor_rect = rect::new_rect(data.x, data.y, data.width, data.height);
        native::window::set_cursor_rect(window.handle, data);
        *argv
    } else {
        rb_sys::Qnil as VALUE
    }
}

unsafe extern "C" fn window_open(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let window = get_window(self_value);
    window.openness = 255;
    native::window::set_openness(window.handle, 255);
    self_value
}

unsafe extern "C" fn window_close(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let window = get_window(self_value);
    window.openness = 0;
    native::window::set_openness(window.handle, 0);
    self_value
}

unsafe extern "C" fn window_update(_argc: c_int, _argv: *const VALUE, _self_value: VALUE) -> VALUE {
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_native_id(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    int_to_value(get_window(self_value).handle as i64)
}

fn apply_all(window: &WindowValue) {
    native::window::set_viewport(window.handle, viewport_handle(window.viewport));
    native::window::set_windowskin(window.handle, bitmap_handle(window.windowskin));
    native::window::set_contents(window.handle, bitmap_handle(window.contents));
    native::window::set_x(window.handle, window.x);
    native::window::set_y(window.handle, window.y);
    native::window::set_z(window.handle, window.z);
    native::window::set_width(window.handle, window.width);
    native::window::set_height(window.handle, window.height);
    native::window::set_ox(window.handle, window.ox);
    native::window::set_oy(window.handle, window.oy);
    native::window::set_opacity(window.handle, window.opacity);
    native::window::set_back_opacity(window.handle, window.back_opacity);
    native::window::set_contents_opacity(window.handle, window.contents_opacity);
    native::window::set_openness(window.handle, window.openness);
    native::window::set_visible(window.handle, window.visible);
    native::window::set_active(window.handle, window.active);
    native::window::set_pause(window.handle, window.pause);
    native::window::set_tone(window.handle, tone_data(window.tone));
    native::window::set_color(window.handle, get_color_data(window.color));
    let rect = rect::rect_data(window.cursor_rect).unwrap_or_else(|| RectData::new(0, 0, 0, 0));
    native::window::set_cursor_rect(window.handle, rect);
}
