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
const SET_BUSH_OPACITY_NAME: &[u8] = b"sprite_set_bush_opacity\0";
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
    pub bush_opacity: i32,
    pub opacity: i32,
    pub blend_type: i32,
    pub visible: bool,
    pub src_rect: RectData,
    pub color: ColorData,
    pub tone: ToneData,
    pub disposed: bool,
    pub flash: Option<SpriteFlashState>,
}

#[derive(Clone, Debug)]
pub struct SpriteFlashState {
    pub color: ColorData,
    alpha: f32,
    duration: i32,
    counter: i32,
    pub empty: bool,
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
            bush_opacity: 128,
            opacity: 255,
            blend_type: 0,
            visible: true,
            src_rect: RectData::default(),
            color: ColorData::default(),
            tone: ToneData::default(),
            disposed: false,
            flash: None,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe { define_sprite_api() }
}

pub fn snapshot() -> Vec<(u32, SpriteData)> {
    SPRITES.snapshot()
}

pub fn create(viewport: Option<u32>) -> u32 {
    let mut data = SpriteData::default();
    data.viewport_id = viewport;
    SPRITES.insert(data)
}

pub fn dispose(id: u32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.disposed = true;
    });
}

pub fn set_viewport(id: u32, viewport: Option<u32>) {
    SPRITES.with_mut(id, |sprite| {
        sprite.viewport_id = viewport;
    });
}

pub fn set_bitmap(id: u32, bitmap: Option<u32>) {
    SPRITES.with_mut(id, |sprite| {
        sprite.bitmap_id = bitmap;
    });
}

pub fn set_x(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.x = value;
    });
}

pub fn set_y(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.y = value;
    });
}

pub fn set_z(id: u32, value: i32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.z = value;
    });
}

pub fn set_ox(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.ox = value;
    });
}

pub fn set_oy(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.oy = value;
    });
}

pub fn set_zoom_x(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.zoom_x = value;
    });
}

pub fn set_zoom_y(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.zoom_y = value;
    });
}

pub fn set_angle(id: u32, value: f32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.angle = value;
    });
}

pub fn set_mirror(id: u32, value: bool) {
    SPRITES.with_mut(id, |sprite| {
        sprite.mirror = value;
    });
}

pub fn set_bush_depth(id: u32, value: i32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.bush_depth = value;
    });
}

pub fn set_bush_opacity(id: u32, value: i32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.bush_opacity = value;
    });
}

pub fn set_opacity(id: u32, value: i32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.opacity = value;
    });
}

pub fn set_blend_type(id: u32, value: i32) {
    SPRITES.with_mut(id, |sprite| {
        sprite.blend_type = value;
    });
}

pub fn set_visible(id: u32, value: bool) {
    SPRITES.with_mut(id, |sprite| {
        sprite.visible = value;
    });
}

pub fn set_src_rect(id: u32, rect: RectData) {
    SPRITES.with_mut(id, |sprite| {
        sprite.src_rect = rect;
    });
}

pub fn set_color(id: u32, color: ColorData) {
    SPRITES.with_mut(id, |sprite| {
        sprite.color = color;
    });
}

pub fn set_tone(id: u32, tone: ToneData) {
    SPRITES.with_mut(id, |sprite| {
        sprite.tone = tone;
    });
}

pub fn start_flash(id: u32, color: Option<ColorData>, duration: i32) {
    if duration < 1 {
        return;
    }
    SPRITES.with_mut(id, |sprite| {
        let (color_data, alpha, empty) = match color {
            Some(c) => (c, c.alpha, false),
            None => (ColorData::default(), 0.0, true),
        };
        let state = SpriteFlashState {
            color: color_data,
            alpha,
            duration,
            counter: 0,
            empty,
        };
        sprite.flash = Some(state);
    });
}

pub fn advance_flash(id: u32) {
    SPRITES.with_mut(id, |sprite| {
        if let Some(flash) = sprite.flash.as_mut() {
            flash.counter += 1;
            if flash.counter > flash.duration {
                sprite.flash = None;
                return;
            }
            if flash.empty {
                return;
            }
            let progress = flash.counter as f32 / flash.duration.max(1) as f32;
            let remaining = flash.alpha * (1.0 - progress);
            flash.color.alpha = remaining.max(0.0);
        }
    });
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
        c_name(SET_BUSH_OPACITY_NAME),
        Some(sprite_set_bush_opacity),
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
    let id = create(viewport_id);
    rb_sys::rb_uint2inum(id as usize)
}

unsafe extern "C" fn sprite_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    dispose(id);
    rb_sys::Qnil as VALUE
}

macro_rules! sprite_setter {
    ($name:ident, $setter:ident, $convert:expr) => {
        unsafe extern "C" fn $name(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
            if argc != 2 || argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let args = std::slice::from_raw_parts(argv, 2);
            let id = value_to_i32(args[0]) as u32;
            let value = $convert(args[1]);
            $setter(id, value);
            rb_sys::Qnil as VALUE
        }
    };
}

sprite_setter!(sprite_set_x, set_x, |val| value_to_f32(val));
sprite_setter!(sprite_set_y, set_y, |val| value_to_f32(val));
sprite_setter!(sprite_set_z, set_z, |val| value_to_i32(val));
sprite_setter!(sprite_set_ox, set_ox, |val| value_to_f32(val));
sprite_setter!(sprite_set_oy, set_oy, |val| value_to_f32(val));
sprite_setter!(sprite_set_zoom_x, set_zoom_x, |val| value_to_f32(val));
sprite_setter!(sprite_set_zoom_y, set_zoom_y, |val| value_to_f32(val));
sprite_setter!(sprite_set_angle, set_angle, |val| value_to_f32(val));
sprite_setter!(sprite_set_bush_depth, set_bush_depth, |val| value_to_i32(
    val
));
sprite_setter!(sprite_set_opacity, set_opacity, |val| value_to_i32(val));
sprite_setter!(sprite_set_blend_type, set_blend_type, |val| value_to_i32(
    val
));
sprite_setter!(sprite_set_bush_opacity, set_bush_opacity, |val| {
    value_to_i32(val)
});

unsafe extern "C" fn sprite_set_viewport(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let viewport = value_to_handle(args[1]);
    set_viewport(id, viewport);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_bitmap(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let bitmap = value_to_handle(args[1]);
    set_bitmap(id, bitmap);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_mirror(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let mirror = value_to_bool(args[1]);
    set_mirror(id, mirror);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn sprite_set_visible(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let visible = value_to_bool(args[1]);
    set_visible(id, visible);
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
    set_src_rect(id, rect);
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
    set_color(id, color);
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
    set_tone(id, tone);
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
