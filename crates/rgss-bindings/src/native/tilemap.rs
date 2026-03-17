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
    fn rb_string_value_ptr(value: *mut VALUE) -> *const c_char;
    fn rb_str_strlen(value: VALUE) -> isize;
}

const CREATE_NAME: &[u8] = b"tilemap_create\0";
const DISPOSE_NAME: &[u8] = b"tilemap_dispose\0";
const SET_VIEWPORT_NAME: &[u8] = b"tilemap_set_viewport\0";
const SET_TILESET_NAME: &[u8] = b"tilemap_set_tileset\0";
const SET_AUTOTILE_NAME: &[u8] = b"tilemap_set_autotile\0";
const SET_MAP_DATA_NAME: &[u8] = b"tilemap_set_map_data\0";
const SET_PRIORITIES_NAME: &[u8] = b"tilemap_set_priorities\0";
const SET_OX_NAME: &[u8] = b"tilemap_set_ox\0";
const SET_OY_NAME: &[u8] = b"tilemap_set_oy\0";
const SET_VISIBLE_NAME: &[u8] = b"tilemap_set_visible\0";
const SET_OPACITY_NAME: &[u8] = b"tilemap_set_opacity\0";
const SET_BLEND_TYPE_NAME: &[u8] = b"tilemap_set_blend_type\0";
const SET_COLOR_NAME: &[u8] = b"tilemap_set_color\0";
const SET_TONE_NAME: &[u8] = b"tilemap_set_tone\0";
const SET_FLASH_DATA_NAME: &[u8] = b"tilemap_set_flash_data\0";
const UPDATE_NAME: &[u8] = b"tilemap_update\0";

static TILEMAPS: Lazy<HandleStore<TilemapData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct TilemapData {
    pub viewport_id: Option<u32>,
    pub tileset_id: Option<u32>,
    pub autotile_ids: [Option<u32>; 7],
    pub map: Option<TilemapGrid>,
    pub priorities: Vec<i16>,
    pub flash: Option<TilemapFlash>,
    pub flash_phase: u8,
    pub ox: i32,
    pub oy: i32,
    pub visible: bool,
    pub opacity: i32,
    pub blend_type: i32,
    pub tone: ToneData,
    pub color: ColorData,
    pub disposed: bool,
}

#[derive(Clone, Debug)]
pub struct TilemapGrid {
    pub width: usize,
    pub height: usize,
    pub layers: Vec<Vec<i16>>,
}

#[derive(Clone, Debug)]
pub struct TilemapFlash {
    pub width: usize,
    pub height: usize,
    pub values: Vec<i16>,
}

impl Default for TilemapData {
    fn default() -> Self {
        Self {
            viewport_id: None,
            tileset_id: None,
            autotile_ids: [None; 7],
            map: None,
            priorities: Vec::new(),
            flash: None,
            flash_phase: 0,
            ox: 0,
            oy: 0,
            visible: true,
            opacity: 255,
            blend_type: 0,
            tone: ToneData::default(),
            color: ColorData::default(),
            disposed: false,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe { define_tilemap_api() }
}

pub fn snapshot() -> Vec<(u32, TilemapData)> {
    TILEMAPS.snapshot()
}

fn insert_tilemap(viewport: Option<u32>) -> u32 {
    let mut data = TilemapData::default();
    data.viewport_id = viewport;
    TILEMAPS.insert(data)
}

pub fn create(viewport: Option<u32>) -> u32 {
    insert_tilemap(viewport)
}

pub fn dispose(id: u32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.disposed = true);
}

pub fn set_viewport(id: u32, viewport: Option<u32>) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.viewport_id = viewport);
}

pub fn set_tileset(id: u32, tileset: Option<u32>) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.tileset_id = tileset);
}

pub fn set_autotile(id: u32, index: usize, bitmap: Option<u32>) {
    if index < 7 {
        TILEMAPS.with_mut(id, |tilemap| tilemap.autotile_ids[index] = bitmap);
    }
}

pub fn set_map_data(id: u32, width: i32, height: i32, layers: i32, values: &[i16]) {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let layers = layers.max(1) as usize;
    set_map_data_internal(id, width, height, layers, values);
}

pub fn clear_map_data(id: u32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.map = None);
}

pub fn set_priorities(id: u32, size: i32, values: &[i16]) {
    let size = size.max(0) as usize;
    let mut data = Vec::with_capacity(size);
    for idx in 0..size {
        data.push(*values.get(idx).unwrap_or(&0));
    }
    assign_priorities(id, data);
}

pub fn clear_priorities(id: u32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.priorities.clear());
}

pub fn set_flash_data(id: u32, width: i32, height: i32, values: &[i16]) {
    let width = width.max(0) as usize;
    let height = height.max(0) as usize;
    assign_flash_data(id, width, height, values);
}

pub fn clear_flash_data(id: u32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.flash = None);
}

pub fn set_ox(id: u32, value: i32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.ox = value);
}

pub fn set_oy(id: u32, value: i32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.oy = value);
}

pub fn set_visible(id: u32, value: bool) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.visible = value);
}

pub fn set_opacity(id: u32, value: i32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.opacity = value);
}

pub fn set_blend_type(id: u32, value: i32) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.blend_type = value);
}

pub fn set_color(id: u32, color: ColorData) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.color = color);
}

pub fn set_tone(id: u32, tone: ToneData) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.tone = tone);
}

pub fn update(id: u32) {
    const FLASH_PHASE_STEPS: u8 = 32;
    TILEMAPS.with_mut(id, |tilemap| {
        if FLASH_PHASE_STEPS > 0 {
            tilemap.flash_phase = (tilemap.flash_phase + 1) % FLASH_PHASE_STEPS;
        }
    });
}

fn set_map_data_internal(id: u32, width: usize, height: usize, layers: usize, values: &[i16]) {
    let plane_len = width * height;
    let mut layer_data = Vec::new();
    for layer in 0..layers {
        let start = layer * plane_len;
        let end = start + plane_len;
        let mut plane = Vec::with_capacity(plane_len);
        for idx in start..end {
            plane.push(*values.get(idx).unwrap_or(&0));
        }
        layer_data.push(plane);
    }
    TILEMAPS.with_mut(id, |tilemap| {
        tilemap.map = Some(TilemapGrid {
            width,
            height,
            layers: layer_data,
        });
    });
}

fn assign_priorities(id: u32, values: Vec<i16>) {
    TILEMAPS.with_mut(id, |tilemap| tilemap.priorities = values);
}

fn assign_flash_data(id: u32, width: usize, height: usize, values: &[i16]) {
    TILEMAPS.with_mut(id, |tilemap| {
        if width == 0 || height == 0 {
            tilemap.flash = None;
        } else {
            let total = width * height;
            let mut data = Vec::with_capacity(total);
            for idx in 0..total {
                data.push(*values.get(idx).unwrap_or(&0));
            }
            tilemap.flash = Some(TilemapFlash {
                width,
                height,
                values: data,
            });
        }
    });
}
unsafe fn define_tilemap_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(CREATE_NAME), Some(tilemap_create), 1);
    rb_define_module_function(native, c_name(DISPOSE_NAME), Some(tilemap_dispose), 1);
    rb_define_module_function(
        native,
        c_name(SET_VIEWPORT_NAME),
        Some(tilemap_set_viewport),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_TILESET_NAME),
        Some(tilemap_set_tileset),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_AUTOTILE_NAME),
        Some(tilemap_set_autotile),
        3,
    );
    rb_define_module_function(
        native,
        c_name(SET_MAP_DATA_NAME),
        Some(tilemap_set_map_data),
        5,
    );
    rb_define_module_function(
        native,
        c_name(SET_PRIORITIES_NAME),
        Some(tilemap_set_priorities),
        3,
    );
    rb_define_module_function(native, c_name(SET_OX_NAME), Some(tilemap_set_ox), 2);
    rb_define_module_function(native, c_name(SET_OY_NAME), Some(tilemap_set_oy), 2);
    rb_define_module_function(
        native,
        c_name(SET_VISIBLE_NAME),
        Some(tilemap_set_visible),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_OPACITY_NAME),
        Some(tilemap_set_opacity),
        2,
    );
    rb_define_module_function(
        native,
        c_name(SET_BLEND_TYPE_NAME),
        Some(tilemap_set_blend_type),
        2,
    );
    rb_define_module_function(native, c_name(SET_COLOR_NAME), Some(tilemap_set_color), 5);
    rb_define_module_function(native, c_name(SET_TONE_NAME), Some(tilemap_set_tone), 5);
    rb_define_module_function(
        native,
        c_name(SET_FLASH_DATA_NAME),
        Some(tilemap_set_flash_data),
        4,
    );
    rb_define_module_function(native, c_name(UPDATE_NAME), Some(tilemap_update), 1);
    Ok(())
}

unsafe extern "C" fn tilemap_create(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let viewport = value_to_handle(*argv);
    let id = insert_tilemap(viewport);
    rb_sys::rb_uint2inum(id as usize)
}

unsafe extern "C" fn tilemap_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    dispose(id);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_viewport(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let viewport = value_to_handle(args[1]);
    set_viewport(id, viewport);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_tileset(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let bitmap = value_to_handle(args[1]);
    set_tileset(id, bitmap);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_autotile(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 3 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 3);
    let id = value_to_i32(args[0]) as u32;
    let index = value_to_i32(args[1]).max(0) as usize;
    let handle = value_to_handle(args[2]);
    set_autotile(id, index, handle);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_map_data(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 5 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 5);
    let id = value_to_i32(args[0]) as u32;
    let width = value_to_i32(args[1]).max(1) as usize;
    let height = value_to_i32(args[2]).max(1) as usize;
    let layers = value_to_i32(args[3]).max(1) as usize;
    let mut blob = args[4];
    let ptr = rb_string_value_ptr(&mut blob);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let len = rb_str_strlen(blob).max(0) as usize;
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    let total = width * height * layers;
    let mut values = Vec::with_capacity(total);
    for idx in 0..total {
        let offset = idx * 2;
        let value = if offset + 1 < bytes.len() {
            i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
        } else {
            0
        };
        values.push(value);
    }
    let mut layer_data = Vec::new();
    let plane_len = width * height;
    for layer in 0..layers {
        let start = layer * plane_len;
        let end = start + plane_len;
        if end <= values.len() {
            layer_data.push(values[start..end].to_vec());
        }
    }
    set_map_data_internal(id, width, height, layers, &values);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_priorities(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 3 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 3);
    let id = value_to_i32(args[0]) as u32;
    let size = value_to_i32(args[1]).max(0) as usize;
    let mut blob = args[2];
    let ptr = rb_string_value_ptr(&mut blob);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let len = rb_str_strlen(blob).max(0) as usize;
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    let mut values = Vec::with_capacity(size);
    for idx in 0..size {
        let offset = idx * 2;
        let value = if offset + 1 < bytes.len() {
            i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
        } else {
            0
        };
        values.push(value);
    }
    assign_priorities(id, values);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_ox(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let ox = value_to_i32(args[1]);
    set_ox(id, ox);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_oy(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let oy = value_to_i32(args[1]);
    set_oy(id, oy);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_set_visible(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = value_to_i32(args[0]) as u32;
    let visible = value_to_bool(args[1]);
    set_visible(id, visible);
    rb_sys::Qnil as VALUE
}

macro_rules! tilemap_setter {
    ($name:ident, $setter:expr, $convert:expr) => {
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

tilemap_setter!(tilemap_set_opacity, set_opacity, |val| value_to_i32(val));
tilemap_setter!(tilemap_set_blend_type, set_blend_type, |val| value_to_i32(
    val
));

unsafe extern "C" fn tilemap_set_color(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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

unsafe extern "C" fn tilemap_set_tone(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
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

unsafe extern "C" fn tilemap_set_flash_data(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc != 4 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 4);
    let id = value_to_i32(args[0]) as u32;
    let width = value_to_i32(args[1]).max(0) as usize;
    let height = value_to_i32(args[2]).max(0) as usize;
    let mut blob = args[3];
    let ptr = rb_string_value_ptr(&mut blob);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let len = rb_str_strlen(blob).max(0) as usize;
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    let total = width * height;
    let mut values = Vec::with_capacity(total);
    for idx in 0..total {
        let offset = idx * 2;
        let value = if offset + 1 < bytes.len() {
            i16::from_le_bytes([bytes[offset], bytes[offset + 1]])
        } else {
            0
        };
        values.push(value);
    }
    assign_flash_data(id, width, height, &values);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_update(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = value_to_i32(*argv) as u32;
    update(id);
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
