use super::{
    native_module, value_to_bool, value_to_f32, value_to_i32, ColorData, HandleStore, ToneData,
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

const CREATE_NAME: &[u8] = b"plane_create\0";
const DISPOSE_NAME: &[u8] = b"plane_dispose\0";
const SET_VIEWPORT_NAME: &[u8] = b"plane_set_viewport\0";
const SET_BITMAP_NAME: &[u8] = b"plane_set_bitmap\0";
const SET_Z_NAME: &[u8] = b"plane_set_z\0";
const SET_OX_NAME: &[u8] = b"plane_set_ox\0";
const SET_OY_NAME: &[u8] = b"plane_set_oy\0";
const SET_ZOOM_X_NAME: &[u8] = b"plane_set_zoom_x\0";
const SET_ZOOM_Y_NAME: &[u8] = b"plane_set_zoom_y\0";
const SET_OPACITY_NAME: &[u8] = b"plane_set_opacity\0";
const SET_BLEND_TYPE_NAME: &[u8] = b"plane_set_blend_type\0";
const SET_VISIBLE_NAME: &[u8] = b"plane_set_visible\0";
const SET_COLOR_NAME: &[u8] = b"plane_set_color\0";
const SET_TONE_NAME: &[u8] = b"plane_set_tone\0";

static PLANES: Lazy<HandleStore<PlaneData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct PlaneData {
    pub viewport_id: Option<u32>,
    pub bitmap_id: Option<u32>,
    pub z: i32,
    pub ox: f32,
    pub oy: f32,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub opacity: i32,
    pub blend_type: i32,
    pub visible: bool,
    pub color: ColorData,
    pub tone: ToneData,
    pub disposed: bool,
}

impl Default for PlaneData {
    fn default() -> Self {
        Self {
            viewport_id: None,
            bitmap_id: None,
            z: 0,
            ox: 0.0,
            oy: 0.0,
            zoom_x: 1.0,
            zoom_y: 1.0,
            opacity: 255,
            blend_type: 0,
            visible: true,
            color: ColorData::default(),
            tone: ToneData::default(),
            disposed: false,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe { define_plane_api() }
}

pub fn snapshot() -> Vec<(u32, PlaneData)> {
    PLANES.snapshot()
}

pub fn create(viewport: Option<u32>) -> u32 {
    let mut data = PlaneData::default();
    data.viewport_id = viewport;
    PLANES.insert(data)
}

pub fn dispose(id: u32) {
    PLANES.with_mut(id, |plane| {
        plane.disposed = true;
    });
}

pub fn set_viewport(id: u32, viewport: Option<u32>) {
    PLANES.with_mut(id, |plane| plane.viewport_id = viewport);
}

pub fn set_bitmap(id: u32, bitmap: Option<u32>) {
    PLANES.with_mut(id, |plane| plane.bitmap_id = bitmap);
}

pub fn set_z(id: u32, value: i32) {
    PLANES.with_mut(id, |plane| plane.z = value);
}

pub fn set_ox(id: u32, value: f32) {
    PLANES.with_mut(id, |plane| plane.ox = value);
}

pub fn set_oy(id: u32, value: f32) {
    PLANES.with_mut(id, |plane| plane.oy = value);
}

pub fn set_zoom_x(id: u32, value: f32) {
    PLANES.with_mut(id, |plane| plane.zoom_x = value);
}

pub fn set_zoom_y(id: u32, value: f32) {
    PLANES.with_mut(id, |plane| plane.zoom_y = value);
}

pub fn set_opacity(id: u32, value: i32) {
    PLANES.with_mut(id, |plane| plane.opacity = value);
}

pub fn set_blend_type(id: u32, value: i32) {
    PLANES.with_mut(id, |plane| plane.blend_type = value);
}

pub fn set_visible(id: u32, value: bool) {
    PLANES.with_mut(id, |plane| plane.visible = value);
}

pub fn set_color(id: u32, color: ColorData) {
    PLANES.with_mut(id, |plane| plane.color = color);
}

pub fn set_tone(id: u32, tone: ToneData) {
    PLANES.with_mut(id, |plane| plane.tone = tone);
}

unsafe fn define_plane_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(CREATE_NAME), Some(plane_create), -1);
    rb_define_module_function(native, c_name(DISPOSE_NAME), Some(plane_dispose), -1);
    rb_define_module_function(
        native,
        c_name(SET_VIEWPORT_NAME),
        Some(plane_set_viewport),
        -1,
    );
    rb_define_module_function(native, c_name(SET_BITMAP_NAME), Some(plane_set_bitmap), -1);
    rb_define_module_function(native, c_name(SET_Z_NAME), Some(plane_set_z), -1);
    rb_define_module_function(native, c_name(SET_OX_NAME), Some(plane_set_ox), -1);
    rb_define_module_function(native, c_name(SET_OY_NAME), Some(plane_set_oy), -1);
    rb_define_module_function(native, c_name(SET_ZOOM_X_NAME), Some(plane_set_zoom_x), -1);
    rb_define_module_function(native, c_name(SET_ZOOM_Y_NAME), Some(plane_set_zoom_y), -1);
    rb_define_module_function(native, c_name(SET_OPACITY_NAME), Some(plane_set_opacity), -1);
    rb_define_module_function(
        native,
        c_name(SET_BLEND_TYPE_NAME),
        Some(plane_set_blend_type),
        -1,
    );
    rb_define_module_function(native, c_name(SET_VISIBLE_NAME), Some(plane_set_visible), -1);
    rb_define_module_function(native, c_name(SET_COLOR_NAME), Some(plane_set_color), -1);
    rb_define_module_function(native, c_name(SET_TONE_NAME), Some(plane_set_tone), -1);
    Ok(())
}

unsafe extern "C" fn plane_create(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let viewport = value_to_handle(*argv);
    let mut data = PlaneData::default();
    data.viewport_id = viewport;
    let id = PLANES.insert(data);
    rb_sys::rb_uint2inum(id as usize)
}

unsafe extern "C" fn plane_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    PLANES.with_mut(id, |plane| {
        plane.disposed = true;
    });
    rb_sys::Qnil as VALUE
}

macro_rules! plane_setter {
    ($name:ident, $field:ident, $convert:expr) => {
        unsafe extern "C" fn $name(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
            if argc != 2 || argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let args = std::slice::from_raw_parts(argv, 2);
            let id = value_to_i32(args[0]) as u32;
            let value = $convert(args[1]);
            PLANES.with_mut(id, |plane| {
                plane.$field = value;
            });
            rb_sys::Qnil as VALUE
        }
    };
}

plane_setter!(plane_set_z, z, |val| value_to_i32(val));
plane_setter!(plane_set_ox, ox, |val| value_to_f32(val));
plane_setter!(plane_set_oy, oy, |val| value_to_f32(val));
plane_setter!(plane_set_zoom_x, zoom_x, |val| value_to_f32(val));
plane_setter!(plane_set_zoom_y, zoom_y, |val| value_to_f32(val));
plane_setter!(plane_set_opacity, opacity, |val| value_to_i32(val));
plane_setter!(plane_set_blend_type, blend_type, |val| value_to_i32(val));

unsafe extern "C" fn plane_set_viewport(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let viewport = value_to_handle(args[1]);
    PLANES.with_mut(id, |plane| plane.viewport_id = viewport);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn plane_set_bitmap(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let bitmap = value_to_handle(args[1]);
    PLANES.with_mut(id, |plane| plane.bitmap_id = bitmap);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn plane_set_visible(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let visible = value_to_bool(args[1]);
    PLANES.with_mut(id, |plane| plane.visible = visible);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn plane_set_color(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    PLANES.with_mut(id, |plane| plane.color = color);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn plane_set_tone(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    PLANES.with_mut(id, |plane| plane.tone = tone);
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
