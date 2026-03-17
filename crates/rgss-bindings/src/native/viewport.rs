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

const CREATE_NAME: &[u8] = b"viewport_create\0";
const DISPOSE_NAME: &[u8] = b"viewport_dispose\0";
const SET_RECT_NAME: &[u8] = b"viewport_set_rect\0";
const SET_VISIBLE_NAME: &[u8] = b"viewport_set_visible\0";
const SET_Z_NAME: &[u8] = b"viewport_set_z\0";
const SET_OX_NAME: &[u8] = b"viewport_set_ox\0";
const SET_OY_NAME: &[u8] = b"viewport_set_oy\0";
const SET_COLOR_NAME: &[u8] = b"viewport_set_color\0";
const SET_TONE_NAME: &[u8] = b"viewport_set_tone\0";

static VIEWPORTS: Lazy<HandleStore<ViewportData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct ViewportData {
    pub rect: RectData,
    pub ox: i32,
    pub oy: i32,
    pub z: i32,
    pub visible: bool,
    pub color: ColorData,
    pub tone: ToneData,
    pub disposed: bool,
}

impl Default for ViewportData {
    fn default() -> Self {
        Self {
            rect: RectData::default(),
            ox: 0,
            oy: 0,
            z: 0,
            visible: true,
            color: ColorData::default(),
            tone: ToneData::default(),
            disposed: false,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe { define_viewport_api() }
}

pub fn snapshot() -> Vec<(u32, ViewportData)> {
    VIEWPORTS.snapshot()
}

unsafe fn define_viewport_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(CREATE_NAME), Some(viewport_create), 4);
    rb_define_module_function(native, c_name(DISPOSE_NAME), Some(viewport_dispose), 1);
    rb_define_module_function(native, c_name(SET_RECT_NAME), Some(viewport_set_rect), 5);
    rb_define_module_function(
        native,
        c_name(SET_VISIBLE_NAME),
        Some(viewport_set_visible),
        2,
    );
    rb_define_module_function(native, c_name(SET_Z_NAME), Some(viewport_set_z), 2);
    rb_define_module_function(native, c_name(SET_OX_NAME), Some(viewport_set_ox), 2);
    rb_define_module_function(native, c_name(SET_OY_NAME), Some(viewport_set_oy), 2);
    rb_define_module_function(native, c_name(SET_COLOR_NAME), Some(viewport_set_color), 5);
    rb_define_module_function(native, c_name(SET_TONE_NAME), Some(viewport_set_tone), 5);
    Ok(())
}

unsafe extern "C" fn viewport_create(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 4 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 4);
    let rect = RectData::new(
        value_to_i32(args[0]),
        value_to_i32(args[1]),
        value_to_i32(args[2]),
        value_to_i32(args[3]),
    );
    let mut data = ViewportData::default();
    data.rect = rect;
    let id = VIEWPORTS.insert(data);
    rb_sys::rb_uint2inum(id as usize)
}

unsafe extern "C" fn viewport_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    VIEWPORTS.with_mut(id, |vp| {
        vp.disposed = true;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_rect(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    VIEWPORTS.with_mut(id, |vp| {
        vp.rect = rect;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_visible(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_bool(args[1]);
    VIEWPORTS.with_mut(id, |vp| {
        vp.visible = value;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_z(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_i32(args[1]);
    VIEWPORTS.with_mut(id, |vp| {
        vp.z = value;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_ox(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_i32(args[1]);
    VIEWPORTS.with_mut(id, |vp| {
        vp.ox = value;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_oy(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let value = value_to_i32(args[1]);
    VIEWPORTS.with_mut(id, |vp| {
        vp.oy = value;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_color(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    VIEWPORTS.with_mut(id, |vp| {
        vp.color = color;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn viewport_set_tone(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 5 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 5);
    let tone = ToneData::new(
        value_to_f32(args[1]),
        value_to_f32(args[2]),
        value_to_f32(args[3]),
        value_to_f32(args[4]),
    );
    let id = value_to_i32(args[0]) as u32;
    VIEWPORTS.with_mut(id, |vp| {
        vp.tone = tone;
    });
    rb_sys::Qnil as VALUE
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
