use super::{
    native_module, value_to_bool, value_to_f32, value_to_i32, ColorData, HandleStore, RectData,
    ToneData,
};
use anyhow::Result;
use once_cell::sync::Lazy;
use rb_sys::VALUE;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE>,
        argc: c_int,
    );
}

const CREATE_NAME: &[u8] = b"window_create\0";
const DISPOSE_NAME: &[u8] = b"window_dispose\0";
const SET_VIEWPORT_NAME: &[u8] = b"window_set_viewport\0";
const SET_WINDOWS_KIND_NAME: &[u8] = b"window_set_windowskin\0";
const SET_CONTENTS_NAME: &[u8] = b"window_set_contents\0";
const SET_X_NAME: &[u8] = b"window_set_x\0";
const SET_Y_NAME: &[u8] = b"window_set_y\0";
const SET_Z_NAME: &[u8] = b"window_set_z\0";
const SET_WIDTH_NAME: &[u8] = b"window_set_width\0";
const SET_HEIGHT_NAME: &[u8] = b"window_set_height\0";
const SET_OX_NAME: &[u8] = b"window_set_ox\0";
const SET_OY_NAME: &[u8] = b"window_set_oy\0";
const SET_OPACITY_NAME: &[u8] = b"window_set_opacity\0";
const SET_BACK_OPACITY_NAME: &[u8] = b"window_set_back_opacity\0";
const SET_CONTENTS_OPACITY_NAME: &[u8] = b"window_set_contents_opacity\0";
const SET_OPENNESS_NAME: &[u8] = b"window_set_openness\0";
const SET_VISIBLE_NAME: &[u8] = b"window_set_visible\0";
const SET_ACTIVE_NAME: &[u8] = b"window_set_active\0";
const SET_PAUSE_NAME: &[u8] = b"window_set_pause\0";
const SET_TONE_NAME: &[u8] = b"window_set_tone\0";
const SET_COLOR_NAME: &[u8] = b"window_set_color\0";
const SET_CURSOR_RECT_NAME: &[u8] = b"window_set_cursor_rect\0";

static WINDOWS: Lazy<HandleStore<WindowData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct WindowData {
    pub viewport_id: Option<u32>,
    pub windowskin_id: Option<u32>,
    pub contents_id: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub width: i32,
    pub height: i32,
    pub ox: i32,
    pub oy: i32,
    pub opacity: i32,
    pub back_opacity: i32,
    pub contents_opacity: i32,
    pub openness: i32,
    pub visible: bool,
    pub active: bool,
    pub pause: bool,
    pub tone: ToneData,
    pub color: ColorData,
    pub cursor_rect: RectData,
    pub disposed: bool,
}

impl Default for WindowData {
    fn default() -> Self {
        Self {
            viewport_id: None,
            windowskin_id: None,
            contents_id: None,
            x: 0,
            y: 0,
            z: 0,
            width: 32,
            height: 32,
            ox: 0,
            oy: 0,
            opacity: 255,
            back_opacity: 255,
            contents_opacity: 255,
            openness: 255,
            visible: true,
            active: true,
            pause: false,
            tone: ToneData::default(),
            color: ColorData::default(),
            cursor_rect: RectData::default(),
            disposed: false,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe { define_window_api() }
}

pub fn snapshot() -> Vec<(u32, WindowData)> {
    WINDOWS.snapshot()
}

pub fn create() -> u32 {
    WINDOWS.insert(WindowData::default())
}

pub fn dispose(id: u32) {
    WINDOWS.with_mut(id, |window| window.disposed = true);
}

pub fn set_viewport(id: u32, viewport: Option<u32>) {
    WINDOWS.with_mut(id, |window| window.viewport_id = viewport);
}

pub fn set_windowskin(id: u32, bitmap: Option<u32>) {
    WINDOWS.with_mut(id, |window| window.windowskin_id = bitmap);
}

pub fn set_contents(id: u32, bitmap: Option<u32>) {
    WINDOWS.with_mut(id, |window| window.contents_id = bitmap);
}

pub fn set_x(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.x = value);
}

pub fn set_y(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.y = value);
}

pub fn set_z(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.z = value);
}

pub fn set_width(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.width = value.max(0));
}

pub fn set_height(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.height = value.max(0));
}

pub fn set_ox(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.ox = value);
}

pub fn set_oy(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.oy = value);
}

pub fn set_opacity(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.opacity = value);
}

pub fn set_back_opacity(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.back_opacity = value);
}

pub fn set_contents_opacity(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.contents_opacity = value);
}

pub fn set_openness(id: u32, value: i32) {
    WINDOWS.with_mut(id, |window| window.openness = value);
}

pub fn set_visible(id: u32, value: bool) {
    WINDOWS.with_mut(id, |window| window.visible = value);
}

pub fn set_active(id: u32, value: bool) {
    WINDOWS.with_mut(id, |window| window.active = value);
}

pub fn set_pause(id: u32, value: bool) {
    WINDOWS.with_mut(id, |window| window.pause = value);
}

pub fn set_tone(id: u32, tone: ToneData) {
    WINDOWS.with_mut(id, |window| window.tone = tone);
}

pub fn set_color(id: u32, color: ColorData) {
    WINDOWS.with_mut(id, |window| window.color = color);
}

pub fn set_cursor_rect(id: u32, rect: RectData) {
    WINDOWS.with_mut(id, |window| window.cursor_rect = rect);
}

unsafe fn define_window_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(CREATE_NAME), Some(window_create), 0);
    rb_define_module_function(native, c_name(DISPOSE_NAME), Some(window_dispose), 1);
    rb_define_module_function(
        native,
        c_name(SET_VIEWPORT_NAME),
        Some(window_set_viewport),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_WINDOWS_KIND_NAME),
        Some(window_set_windowskin),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_CONTENTS_NAME),
        Some(window_set_contents),
        2,
    );
    rb_define_module_function(native, c_name(SET_X_NAME), Some(window_set_x), 2);
    rb_define_module_function(native, c_name(SET_Y_NAME), Some(window_set_y), 2);
    rb_define_module_function(native, c_name(SET_Z_NAME), Some(window_set_z), 2);
    rb_define_module_function(native, c_name(SET_WIDTH_NAME), Some(window_set_width), 2);
    rb_define_module_function(native, c_name(SET_HEIGHT_NAME), Some(window_set_height), 2);
    rb_define_module_function(native, c_name(SET_OX_NAME), Some(window_set_ox), 2);
    rb_define_module_function(native, c_name(SET_OY_NAME), Some(window_set_oy), 2);
    rb_define_module_function(
        native,
        c_name(SET_OPACITY_NAME),
        Some(window_set_opacity),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_BACK_OPACITY_NAME),
        Some(window_set_back_opacity),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_CONTENTS_OPACITY_NAME),
        Some(window_set_contents_opacity),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_OPENNESS_NAME),
        Some(window_set_openness),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_VISIBLE_NAME),
        Some(window_set_visible),
        2,
    );
    rb_define_module_function(native, c_name(SET_ACTIVE_NAME), Some(window_set_active), 2);
    rb_define_module_function(native, c_name(SET_PAUSE_NAME), Some(window_set_pause), 2);
    rb_define_module_function(native, c_name(SET_TONE_NAME), Some(window_set_tone), 5);
    rb_define_module_function(native, c_name(SET_COLOR_NAME), Some(window_set_color), 5);
    rb_define_module_function(
        native,
        c_name(SET_CURSOR_RECT_NAME),
        Some(window_set_cursor_rect),
        5,
    );
    Ok(())
}

unsafe extern "C" fn window_create(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let id = WINDOWS.insert(WindowData::default());
    rb_sys::rb_uint2inum(id as usize)
}

unsafe extern "C" fn window_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    WINDOWS.with_mut(id, |window| window.disposed = true);
    rb_sys::Qnil as VALUE
}

macro_rules! window_setter {
    ($name:ident, $field:ident, $convert:expr) => {
        unsafe extern "C" fn $name(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
            if argc != 2 || argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let args = std::slice::from_raw_parts(argv, 2);
            let id = value_to_i32(args[0]) as u32;
            let value = $convert(args[1]);
            WINDOWS.with_mut(id, |window| {
                window.$field = value;
            });
            rb_sys::Qnil as VALUE
        }
    };
}

window_setter!(window_set_x, x, |val| value_to_i32(val));
window_setter!(window_set_y, y, |val| value_to_i32(val));
window_setter!(window_set_z, z, |val| value_to_i32(val));
window_setter!(window_set_width, width, |val| value_to_i32(val));
window_setter!(window_set_height, height, |val| value_to_i32(val));
window_setter!(window_set_ox, ox, |val| value_to_i32(val));
window_setter!(window_set_oy, oy, |val| value_to_i32(val));
window_setter!(window_set_opacity, opacity, |val| value_to_i32(val));
window_setter!(window_set_back_opacity, back_opacity, |val| value_to_i32(
    val
));
window_setter!(window_set_contents_opacity, contents_opacity, |val| {
    value_to_i32(val)
});
window_setter!(window_set_openness, openness, |val| value_to_i32(val));

unsafe extern "C" fn window_set_visible(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_bool(args[1]);
    WINDOWS.with_mut(id, |window| window.visible = value);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_active(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_bool(args[1]);
    WINDOWS.with_mut(id, |window| window.active = value);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_pause(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_bool(args[1]);
    WINDOWS.with_mut(id, |window| window.pause = value);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_viewport(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let viewport = value_to_handle(args[1]);
    WINDOWS.with_mut(id, |window| {
        window.viewport_id = viewport;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_windowskin(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let handle = value_to_handle(args[1]);
    WINDOWS.with_mut(id, |window| window.windowskin_id = handle);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_contents(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let handle = value_to_handle(args[1]);
    WINDOWS.with_mut(id, |window| window.contents_id = handle);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_tone(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 5 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 5);
    let id = value_to_i32(args[0]) as u32;
    let tone = ToneData::new(
        value_to_f32(args[1]),
        value_to_f32(args[2]),
        value_to_f32(args[3]),
        value_to_f32(args[4]),
    );
    WINDOWS.with_mut(id, |window| window.tone = tone);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_color(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 5 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 5);
    let id = value_to_i32(args[0]) as u32;
    let color = ColorData::new(
        value_to_f32(args[1]),
        value_to_f32(args[2]),
        value_to_f32(args[3]),
        value_to_f32(args[4]),
    );
    WINDOWS.with_mut(id, |window| window.color = color);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn window_set_cursor_rect(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 5 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 5);
    let id = value_to_i32(args[0]) as u32;
    let rect = RectData::new(
        value_to_i32(args[1]),
        value_to_i32(args[2]),
        value_to_i32(args[3]),
        value_to_i32(args[4]),
    );
    WINDOWS.with_mut(id, |window| window.cursor_rect = rect);
    rb_sys::Qnil as VALUE
}

fn value_to_handle(value: VALUE) -> Option<u32> {
    if value == rb_sys::Qnil as VALUE {
        None
    } else {
        Some(value_to_i32(value) as u32)
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
