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

const CREATE_NAME: &[u8] = b"sprite_create\0";
const DISPOSE_NAME: &[u8] = b"sprite_dispose\0";
const SET_VIEWPORT_NAME: &[u8] = b"sprite_set_viewport\0";
const SET_BITMAP_NAME: &[u8] = b"sprite_set_bitmap\0";
const SET_X_NAME: &[u8] = b"sprite_set_x\0";
const SET_Y_NAME: &[u8] = b"sprite_set_y\0";
const SET_Z_NAME: &[u8] = b"sprite_set_z\0";
const SET_OX_NAME: &[u8] = b"sprite_set_ox\0";
const SET_OY_NAME: &[u8] = b"sprite_set_oy\0";
const SET_ZOOM_X_NAME: &[u8] = b"sprite_set_zoom_x\0";
const SET_ZOOM_Y_NAME: &[u8] = b"sprite_set_zoom_y\0";
const SET_ANGLE_NAME: &[u8] = b"sprite_set_angle\0";
const SET_MIRROR_NAME: &[u8] = b"sprite_set_mirror\0";
const SET_BUSH_DEPTH_NAME: &[u8] = b"sprite_set_bush_depth\0";
const SET_OPACITY_NAME: &[u8] = b"sprite_set_opacity\0";
const SET_BLEND_TYPE_NAME: &[u8] = b"sprite_set_blend_type\0";
const SET_VISIBLE_NAME: &[u8] = b"sprite_set_visible\0";
const SET_SRC_RECT_NAME: &[u8] = b"sprite_set_src_rect\0";
const SET_COLOR_NAME: &[u8] = b"sprite_set_color\0";
const SET_TONE_NAME: &[u8] = b"sprite_set_tone\0";

static SPRITES: Lazy<HandleStore<SpriteData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct SpriteData {
    pub viewport_id: Option<u32>,
    pub bitmap_id: Option<u32>,
    pub x: f32,
    pub y: f32,
    pub z: i32,
    pub ox: f32,
    pub oy: f32,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub angle: f32,
    pub mirror: bool,
    pub bush_depth: i32,
    pub opacity: i32,
    pub blend_type: i32,
    pub visible: bool,
    pub src_rect: RectData,
    pub color: ColorData,
    pub tone: ToneData,
    pub disposed: bool,
}

impl Default for SpriteData {
    fn default() -> Self {
        Self {
            viewport_id: None,
            bitmap_id: None,
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
            opacity: 255,
            blend_type: 0,
            visible: true,
            src_rect: RectData::default(),
            color: ColorData::default(),
            tone: ToneData::default(),
            disposed: false,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe { define_sprite_api() }
}

pub fn snapshot() -> Vec<(u32, SpriteData)> {
    SPRITES.snapshot()
}

unsafe fn define_sprite_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(CREATE_NAME), Some(sprite_create), 1);
    rb_define_module_function(native, c_name(DISPOSE_NAME), Some(sprite_dispose), 1);
    rb_define_module_function(
        native,
        c_name(SET_VIEWPORT_NAME),
        Some(sprite_set_viewport),
        2,
    );
    rb_define_module_function(native, c_name(SET_BITMAP_NAME), Some(sprite_set_bitmap), 2);
    rb_define_module_function(native, c_name(SET_X_NAME), Some(sprite_set_x), 2);
    rb_define_module_function(native, c_name(SET_Y_NAME), Some(sprite_set_y), 2);
    rb_define_module_function(native, c_name(SET_Z_NAME), Some(sprite_set_z), 2);
    rb_define_module_function(native, c_name(SET_OX_NAME), Some(sprite_set_ox), 2);
    rb_define_module_function(native, c_name(SET_OY_NAME), Some(sprite_set_oy), 2);
    rb_define_module_function(native, c_name(SET_ZOOM_X_NAME), Some(sprite_set_zoom_x), 2);
    rb_define_module_function(native, c_name(SET_ZOOM_Y_NAME), Some(sprite_set_zoom_y), 2);
    rb_define_module_function(native, c_name(SET_ANGLE_NAME), Some(sprite_set_angle), 2);
    rb_define_module_function(native, c_name(SET_MIRROR_NAME), Some(sprite_set_mirror), 2);
    rb_define_module_function(
        native,
        c_name(SET_BUSH_DEPTH_NAME),
        Some(sprite_set_bush_depth),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_OPACITY_NAME),
        Some(sprite_set_opacity),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_BLEND_TYPE_NAME),
        Some(sprite_set_blend_type),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_VISIBLE_NAME),
        Some(sprite_set_visible),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_SRC_RECT_NAME),
        Some(sprite_set_src_rect),
        5,
    );
    rb_define_module_function(native, c_name(SET_COLOR_NAME), Some(sprite_set_color), 5);
    rb_define_module_function(native, c_name(SET_TONE_NAME), Some(sprite_set_tone), 5);
    Ok(())
}

unsafe extern "C" fn sprite_create(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let viewport_id = value_to_handle(*argv);
    let mut data = SpriteData::default();
    data.viewport_id = viewport_id;
    let id = SPRITES.insert(data);
    rb_sys::rb_uint2inum(id as usize)
}

unsafe extern "C" fn sprite_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    SPRITES.with_mut(id, |sprite| {
        sprite.disposed = true;
    });
    rb_sys::Qnil as VALUE
}

macro_rules! sprite_setter {
    ($name:ident, $field:ident, $convert:expr) => {
        unsafe extern "C" fn $name(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
            if argc != 2 || argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let args = std::slice::from_raw_parts(argv, 2);
            let id = value_to_i32(args[0]) as u32;
            let value = $convert(args[1]);
            SPRITES.with_mut(id, |sprite| {
                sprite.$field = value;
            });
            rb_sys::Qnil as VALUE
        }
    };
}

sprite_setter!(sprite_set_x, x, |val| value_to_f32(val));
sprite_setter!(sprite_set_y, y, |val| value_to_f32(val));
sprite_setter!(sprite_set_z, z, |val| value_to_i32(val));
sprite_setter!(sprite_set_ox, ox, |val| value_to_f32(val));
sprite_setter!(sprite_set_oy, oy, |val| value_to_f32(val));
sprite_setter!(sprite_set_zoom_x, zoom_x, |val| value_to_f32(val));
sprite_setter!(sprite_set_zoom_y, zoom_y, |val| value_to_f32(val));
sprite_setter!(sprite_set_angle, angle, |val| value_to_f32(val));
sprite_setter!(sprite_set_bush_depth, bush_depth, |val| value_to_i32(val));
sprite_setter!(sprite_set_opacity, opacity, |val| value_to_i32(val));
sprite_setter!(sprite_set_blend_type, blend_type, |val| value_to_i32(val));

unsafe extern "C" fn sprite_set_viewport(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let viewport = value_to_handle(args[1]);
    SPRITES.with_mut(id, |sprite| {
        sprite.viewport_id = viewport;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_bitmap(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let bitmap = value_to_handle(args[1]);
    SPRITES.with_mut(id, |sprite| {
        sprite.bitmap_id = bitmap;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_mirror(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let mirror = value_to_bool(args[1]);
    SPRITES.with_mut(id, |sprite| {
        sprite.mirror = mirror;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_visible(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let visible = value_to_bool(args[1]);
    SPRITES.with_mut(id, |sprite| {
        sprite.visible = visible;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_src_rect(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    SPRITES.with_mut(id, |sprite| {
        sprite.src_rect = rect;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_color(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    SPRITES.with_mut(id, |sprite| {
        sprite.color = color;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_tone(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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
    SPRITES.with_mut(id, |sprite| {
        sprite.tone = tone;
    });
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
