use super::{
    bitmap::{bitmap_handle, is_bitmap},
    color::{clone_color, get_color_data, is_color, new_color},
    common::{
        bool_to_value, define_method, get_typed_data, install_allocator, int_to_value,
        wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
    table::{table_snapshot, TableSnapshot},
    tone::{clone_tone, is_tone, new_tone, tone_data},
    viewport::{is_viewport, viewport_handle},
};
use crate::native::{self, value_to_bool, value_to_i32};
use anyhow::Result;
use once_cell::sync::OnceCell;
use rb_sys::{
    bindings::{rb_Array, rb_ary_entry, rb_gc_mark},
    macros::RARRAY_LEN,
    VALUE,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::{c_int, c_long},
    slice,
};
use tracing::warn;

extern "C" {
    fn rb_yield(arg1: VALUE) -> VALUE;
    fn rb_block_given_p() -> c_int;
}

const TILEMAP_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Tilemap\0") };
const TILEMAP_STRUCT_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Tilemap\0") };
const AUTOTILE_CLASS_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::TilemapAutotiles\0") };
const AUTOTILE_STRUCT_NAME: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::TilemapAutotilesStruct\0") };

static TILEMAP_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(TILEMAP_STRUCT_NAME)
        .mark(tilemap_mark)
        .free(tilemap_free)
});
static TILEMAP_CLASS: OnceCell<VALUE> = OnceCell::new();

static AUTOTILE_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(AUTOTILE_STRUCT_NAME)
        .mark(autotile_mark)
        .free(autotile_free)
});
static AUTOTILE_CLASS: OnceCell<VALUE> = OnceCell::new();

#[derive(Clone)]
struct TilemapValue {
    handle: u32,
    disposed: bool,
    viewport: VALUE,
    tileset: VALUE,
    autotiles: [VALUE; 7],
    autotile_proxy: VALUE,
    map_data: VALUE,
    priorities: VALUE,
    flash_data: VALUE,
    tone: VALUE,
    color: VALUE,
    ox: i32,
    oy: i32,
    visible: bool,
    opacity: i32,
    blend_type: i32,
}

impl Default for TilemapValue {
    fn default() -> Self {
        Self {
            handle: 0,
            disposed: true,
            viewport: rb_sys::Qnil as VALUE,
            tileset: rb_sys::Qnil as VALUE,
            autotiles: [rb_sys::Qnil as VALUE; 7],
            autotile_proxy: rb_sys::Qnil as VALUE,
            map_data: rb_sys::Qnil as VALUE,
            priorities: rb_sys::Qnil as VALUE,
            flash_data: rb_sys::Qnil as VALUE,
            tone: rb_sys::Qnil as VALUE,
            color: rb_sys::Qnil as VALUE,
            ox: 0,
            oy: 0,
            visible: true,
            opacity: 255,
            blend_type: 0,
        }
    }
}

#[derive(Clone)]
struct AutotileProxyValue {
    tilemap: VALUE,
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(TILEMAP_CLASS_NAME, None);
        let _ = TILEMAP_CLASS.set(klass);
        install_allocator(klass, Some(tilemap_allocate));
        define_method(klass, cstr(b"initialize\0"), tilemap_initialize, -1);
        define_method(klass, cstr(b"dispose\0"), tilemap_dispose, 0);
        define_method(klass, cstr(b"disposed?\0"), tilemap_disposed_q, 0);
        define_method(klass, cstr(b"viewport\0"), tilemap_get_viewport, 0);
        define_method(klass, cstr(b"viewport=\0"), tilemap_set_viewport, -1);
        define_method(klass, cstr(b"tileset\0"), tilemap_get_tileset, 0);
        define_method(klass, cstr(b"tileset=\0"), tilemap_set_tileset, -1);
        define_method(klass, cstr(b"autotiles\0"), tilemap_get_autotiles, 0);
        define_method(klass, cstr(b"autotiles=\0"), tilemap_set_autotiles, -1);
        define_method(klass, cstr(b"bitmaps\0"), tilemap_get_autotiles, 0);
        define_method(klass, cstr(b"bitmaps=\0"), tilemap_set_autotiles, -1);
        define_method(klass, cstr(b"map_data\0"), tilemap_get_map_data, 0);
        define_method(klass, cstr(b"map_data=\0"), tilemap_set_map_data, -1);
        define_method(klass, cstr(b"priorities\0"), tilemap_get_priorities, 0);
        define_method(klass, cstr(b"priorities=\0"), tilemap_set_priorities, -1);
        define_method(klass, cstr(b"flash_data\0"), tilemap_get_flash_data, 0);
        define_method(klass, cstr(b"flash_data=\0"), tilemap_set_flash_data, -1);
        define_method(klass, cstr(b"ox\0"), tilemap_get_ox, 0);
        define_method(klass, cstr(b"ox=\0"), tilemap_set_ox, -1);
        define_method(klass, cstr(b"oy\0"), tilemap_get_oy, 0);
        define_method(klass, cstr(b"oy=\0"), tilemap_set_oy, -1);
        define_method(klass, cstr(b"visible\0"), tilemap_get_visible, 0);
        define_method(klass, cstr(b"visible=\0"), tilemap_set_visible, -1);
        define_method(klass, cstr(b"opacity\0"), tilemap_get_opacity, 0);
        define_method(klass, cstr(b"opacity=\0"), tilemap_set_opacity, -1);
        define_method(klass, cstr(b"blend_type\0"), tilemap_get_blend_type, 0);
        define_method(klass, cstr(b"blend_type=\0"), tilemap_set_blend_type, -1);
        define_method(klass, cstr(b"tone\0"), tilemap_get_tone, 0);
        define_method(klass, cstr(b"tone=\0"), tilemap_set_tone, -1);
        define_method(klass, cstr(b"color\0"), tilemap_get_color, 0);
        define_method(klass, cstr(b"color=\0"), tilemap_set_color, -1);
        define_method(klass, cstr(b"update\0"), tilemap_update, 0);
        define_method(klass, cstr(b"native_id\0"), tilemap_native_id, 0);
        init_autotile_proxy()?;
    }
    Ok(())
}

fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}

unsafe extern "C" fn tilemap_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, TilemapValue::default(), TILEMAP_TYPE.as_rb_type())
}

unsafe extern "C" fn tilemap_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let tilemap = &*(ptr as *mut TilemapValue);
    for value in tilemap.autotiles.iter() {
        if *value != rb_sys::Qnil as VALUE {
            rb_gc_mark(*value);
        }
    }
    for value in [
        tilemap.viewport,
        tilemap.tileset,
        tilemap.autotile_proxy,
        tilemap.map_data,
        tilemap.priorities,
        tilemap.flash_data,
        tilemap.tone,
        tilemap.color,
    ] {
        if value != rb_sys::Qnil as VALUE {
            rb_gc_mark(value);
        }
    }
}

unsafe extern "C" fn tilemap_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = Box::<TilemapValue>::from_raw(ptr as *mut TilemapValue);
    if !value.disposed && value.handle != 0 {
        native::tilemap::dispose(value.handle);
    }
}

fn get_tilemap(value: VALUE) -> &'static mut TilemapValue {
    unsafe { get_typed_data(value, TILEMAP_TYPE.as_rb_type()) }
        .expect("Tilemap missing native data")
}

unsafe extern "C" fn tilemap_initialize(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let viewport_value = args.get(0).copied().unwrap_or(rb_sys::Qnil as VALUE);
    if viewport_value != rb_sys::Qnil as VALUE && !is_viewport(viewport_value) {
        warn!(target: "rgss", "Tilemap#initialize received non-Viewport");
    }
    let handle = native::tilemap::create(viewport_handle(viewport_value));
    let tilemap = get_tilemap(self_value);
    tilemap.handle = handle;
    tilemap.disposed = false;
    tilemap.viewport = viewport_value;
    tilemap.tileset = rb_sys::Qnil as VALUE;
    tilemap.autotiles = [rb_sys::Qnil as VALUE; 7];
    tilemap.autotile_proxy = rb_sys::Qnil as VALUE;
    tilemap.map_data = rb_sys::Qnil as VALUE;
    tilemap.priorities = rb_sys::Qnil as VALUE;
    tilemap.flash_data = rb_sys::Qnil as VALUE;
    tilemap.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    tilemap.color = new_color(0.0, 0.0, 0.0, 0.0);
    tilemap.ox = 0;
    tilemap.oy = 0;
    tilemap.visible = true;
    tilemap.opacity = 255;
    tilemap.blend_type = 0;
    apply_all(tilemap);
    self_value
}

unsafe extern "C" fn tilemap_dispose(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let tilemap = get_tilemap(self_value);
    if !tilemap.disposed && tilemap.handle != 0 {
        native::tilemap::dispose(tilemap.handle);
        tilemap.disposed = true;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_disposed_q(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    bool_to_value(get_tilemap(self_value).disposed)
}

macro_rules! tilemap_int_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            int_to_value(get_tilemap(self_value).$field as i64)
        }
    };
}

macro_rules! tilemap_bool_getter {
    ($name:ident, $field:ident) => {
        unsafe extern "C" fn $name(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
            bool_to_value(get_tilemap(self_value).$field)
        }
    };
}

macro_rules! tilemap_int_setter {
    ($name:ident, $field:ident, $setter:expr) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_i32(*argv);
            let tilemap = get_tilemap(self_value);
            tilemap.$field = value;
            $setter(tilemap.handle, value);
            *argv
        }
    };
}

macro_rules! tilemap_bool_setter {
    ($name:ident, $field:ident, $setter:expr) => {
        unsafe extern "C" fn $name(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
            if argv.is_null() {
                return rb_sys::Qnil as VALUE;
            }
            let value = value_to_bool(*argv);
            let tilemap = get_tilemap(self_value);
            tilemap.$field = value;
            $setter(tilemap.handle, value);
            *argv
        }
    };
}

tilemap_int_getter!(tilemap_get_ox, ox);
tilemap_int_getter!(tilemap_get_oy, oy);
tilemap_int_getter!(tilemap_get_opacity, opacity);
tilemap_int_getter!(tilemap_get_blend_type, blend_type);
tilemap_bool_getter!(tilemap_get_visible, visible);

tilemap_int_setter!(tilemap_set_ox, ox, native::tilemap::set_ox);
tilemap_int_setter!(tilemap_set_oy, oy, native::tilemap::set_oy);
tilemap_int_setter!(tilemap_set_opacity, opacity, native::tilemap::set_opacity);
tilemap_int_setter!(
    tilemap_set_blend_type,
    blend_type,
    native::tilemap::set_blend_type
);
tilemap_bool_setter!(tilemap_set_visible, visible, native::tilemap::set_visible);

unsafe extern "C" fn tilemap_get_viewport(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_tilemap(self_value).viewport
}

unsafe extern "C" fn tilemap_set_viewport(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_viewport(value) {
        warn!(target: "rgss", "Tilemap#viewport= received non-Viewport");
        return rb_sys::Qnil as VALUE;
    }
    let handle = viewport_handle(value);
    let tilemap = get_tilemap(self_value);
    tilemap.viewport = value;
    native::tilemap::set_viewport(tilemap.handle, handle);
    value
}

unsafe extern "C" fn tilemap_get_tileset(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_tilemap(self_value).tileset
}

unsafe extern "C" fn tilemap_set_tileset(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if value != rb_sys::Qnil as VALUE && !is_bitmap(value) {
        warn!(target: "rgss", "Tilemap#tileset= received non-Bitmap");
        return rb_sys::Qnil as VALUE;
    }
    let handle = bitmap_handle(value);
    let tilemap = get_tilemap(self_value);
    tilemap.tileset = if handle.is_some() {
        value
    } else {
        rb_sys::Qnil as VALUE
    };
    native::tilemap::set_tileset(tilemap.handle, handle);
    value
}

unsafe extern "C" fn tilemap_get_autotiles(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let tilemap = get_tilemap(self_value);
    if tilemap.autotile_proxy == rb_sys::Qnil as VALUE {
        tilemap.autotile_proxy = new_autotile_proxy(self_value);
    }
    tilemap.autotile_proxy
}

unsafe extern "C" fn tilemap_set_autotiles(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let array = rb_Array(*argv);
    let tilemap = get_tilemap(self_value);
    let len = RARRAY_LEN(array) as usize;
    for index in 0..7 {
        let value = if index < len {
            unsafe { rb_ary_entry(array, index as c_long) }
        } else {
            rb_sys::Qnil as VALUE
        };
        set_autotile_value(tilemap, index, value);
    }
    tilemap.autotile_proxy = rb_sys::Qnil as VALUE;
    *argv
}

unsafe extern "C" fn tilemap_get_map_data(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_tilemap(self_value).map_data
}

unsafe extern "C" fn tilemap_set_map_data(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let tilemap = get_tilemap(self_value);
    if value == rb_sys::Qnil as VALUE {
        tilemap.map_data = rb_sys::Qnil as VALUE;
        native::tilemap::clear_map_data(tilemap.handle);
        return value;
    }
    match table_snapshot(value) {
        Some(snapshot) => {
            apply_map_snapshot(tilemap.handle, &snapshot);
            tilemap.map_data = value;
            value
        }
        None => {
            warn!(target: "rgss", "Tilemap#map_data= expected Table");
            rb_sys::Qnil as VALUE
        }
    }
}

unsafe extern "C" fn tilemap_get_priorities(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_tilemap(self_value).priorities
}

unsafe extern "C" fn tilemap_set_priorities(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let tilemap = get_tilemap(self_value);
    if value == rb_sys::Qnil as VALUE {
        tilemap.priorities = rb_sys::Qnil as VALUE;
        native::tilemap::clear_priorities(tilemap.handle);
        return value;
    }
    match table_snapshot(value) {
        Some(snapshot) => {
            native::tilemap::set_priorities(
                tilemap.handle,
                snapshot.data.len() as i32,
                &snapshot.data,
            );
            tilemap.priorities = value;
            value
        }
        None => {
            warn!(target: "rgss", "Tilemap#priorities= expected Table");
            rb_sys::Qnil as VALUE
        }
    }
}

unsafe extern "C" fn tilemap_get_flash_data(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    get_tilemap(self_value).flash_data
}

unsafe extern "C" fn tilemap_set_flash_data(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let tilemap = get_tilemap(self_value);
    if value == rb_sys::Qnil as VALUE {
        tilemap.flash_data = rb_sys::Qnil as VALUE;
        native::tilemap::clear_flash_data(tilemap.handle);
        return value;
    }
    match table_snapshot(value) {
        Some(snapshot) => {
            native::tilemap::set_flash_data(
                tilemap.handle,
                snapshot.xsize,
                snapshot.ysize,
                &snapshot.data,
            );
            tilemap.flash_data = value;
            value
        }
        None => {
            warn!(target: "rgss", "Tilemap#flash_data= expected Table");
            rb_sys::Qnil as VALUE
        }
    }
}

unsafe extern "C" fn tilemap_get_tone(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let tilemap = get_tilemap(self_value);
    if tilemap.tone == rb_sys::Qnil as VALUE {
        tilemap.tone = new_tone(0.0, 0.0, 0.0, 0.0);
    }
    tilemap.tone
}

unsafe extern "C" fn tilemap_set_tone(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    if !is_tone(value) {
        return rb_sys::Qnil as VALUE;
    }
    let tilemap = get_tilemap(self_value);
    let tone = clone_tone(value);
    tilemap.tone = tone;
    native::tilemap::set_tone(tilemap.handle, tone_data(tone));
    value
}

unsafe extern "C" fn tilemap_get_color(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let tilemap = get_tilemap(self_value);
    if tilemap.color == rb_sys::Qnil as VALUE {
        tilemap.color = new_color(0.0, 0.0, 0.0, 0.0);
    }
    tilemap.color
}

unsafe extern "C" fn tilemap_set_color(
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
    let tilemap = get_tilemap(self_value);
    let color = clone_color(value);
    tilemap.color = color;
    native::tilemap::set_color(tilemap.handle, get_color_data(color));
    value
}

unsafe extern "C" fn tilemap_update(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let tilemap = get_tilemap(self_value);
    native::tilemap::update(tilemap.handle);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn tilemap_native_id(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    int_to_value(get_tilemap(self_value).handle as i64)
}

fn apply_map_snapshot(handle: u32, snapshot: &TableSnapshot) {
    native::tilemap::set_map_data(
        handle,
        snapshot.xsize,
        snapshot.ysize,
        snapshot.zsize,
        &snapshot.data,
    );
}

fn apply_all(tilemap: &TilemapValue) {
    native::tilemap::set_viewport(tilemap.handle, viewport_handle(tilemap.viewport));
    native::tilemap::set_tileset(tilemap.handle, bitmap_handle(tilemap.tileset));
    for (index, value) in tilemap.autotiles.iter().enumerate() {
        native::tilemap::set_autotile(tilemap.handle, index, bitmap_handle(*value));
    }
    if tilemap.map_data != rb_sys::Qnil as VALUE {
        if let Some(snapshot) = table_snapshot(tilemap.map_data) {
            apply_map_snapshot(tilemap.handle, &snapshot);
        }
    }
    if tilemap.priorities != rb_sys::Qnil as VALUE {
        if let Some(snapshot) = table_snapshot(tilemap.priorities) {
            native::tilemap::set_priorities(
                tilemap.handle,
                snapshot.data.len() as i32,
                &snapshot.data,
            );
        }
    }
    if tilemap.flash_data != rb_sys::Qnil as VALUE {
        if let Some(snapshot) = table_snapshot(tilemap.flash_data) {
            native::tilemap::set_flash_data(
                tilemap.handle,
                snapshot.xsize,
                snapshot.ysize,
                &snapshot.data,
            );
        }
    }
    native::tilemap::set_ox(tilemap.handle, tilemap.ox);
    native::tilemap::set_oy(tilemap.handle, tilemap.oy);
    native::tilemap::set_visible(tilemap.handle, tilemap.visible);
    native::tilemap::set_opacity(tilemap.handle, tilemap.opacity);
    native::tilemap::set_blend_type(tilemap.handle, tilemap.blend_type);
    native::tilemap::set_tone(tilemap.handle, tone_data(tilemap.tone));
    native::tilemap::set_color(tilemap.handle, get_color_data(tilemap.color));
}

fn set_autotile_value(tilemap: &mut TilemapValue, index: usize, value: VALUE) -> VALUE {
    if index >= tilemap.autotiles.len() {
        return rb_sys::Qnil as VALUE;
    }
    if value != rb_sys::Qnil as VALUE && !is_bitmap(value) {
        warn!(target: "rgss", "Tilemap autotile assignment expected Bitmap or nil");
        return rb_sys::Qnil as VALUE;
    }
    let handle = bitmap_handle(value);
    tilemap.autotiles[index] = if handle.is_some() {
        value
    } else {
        rb_sys::Qnil as VALUE
    };
    native::tilemap::set_autotile(tilemap.handle, index, handle);
    tilemap.autotiles[index]
}

fn new_autotile_proxy(tilemap_value: VALUE) -> VALUE {
    unsafe {
        let klass = *AUTOTILE_CLASS
            .get()
            .expect("autotile class not initialised");
        let value = wrap_typed_data(
            klass,
            AutotileProxyValue {
                tilemap: tilemap_value,
            },
            AUTOTILE_TYPE.as_rb_type(),
        );
        value
    }
}

fn init_autotile_proxy() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(AUTOTILE_CLASS_NAME, None);
        let _ = AUTOTILE_CLASS.set(klass);
        install_allocator(klass, Some(autotile_allocate));
        define_method(klass, cstr(b"[]\0"), autotile_get, -1);
        define_method(klass, cstr(b"[]=\0"), autotile_set, -1);
        define_method(klass, cstr(b"each\0"), autotile_each, 0);
        define_method(klass, cstr(b"length\0"), autotile_length, 0);
        define_method(klass, cstr(b"size\0"), autotile_length, 0);
        define_method(klass, cstr(b"replace\0"), autotile_replace, -1);
    }
    Ok(())
}

unsafe extern "C" fn autotile_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(
        klass,
        AutotileProxyValue {
            tilemap: rb_sys::Qnil as VALUE,
        },
        AUTOTILE_TYPE.as_rb_type(),
    )
}

unsafe extern "C" fn autotile_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let proxy = &*(ptr as *mut AutotileProxyValue);
    if proxy.tilemap != rb_sys::Qnil as VALUE {
        rb_gc_mark(proxy.tilemap);
    }
}

unsafe extern "C" fn autotile_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    drop(Box::<AutotileProxyValue>::from_raw(
        ptr as *mut AutotileProxyValue,
    ));
}

fn get_autotile_proxy(value: VALUE) -> &'static mut AutotileProxyValue {
    unsafe { get_typed_data(value, AUTOTILE_TYPE.as_rb_type()) }
        .expect("Tilemap autotiles missing data")
}

fn proxy_tilemap(value: VALUE) -> Option<&'static mut TilemapValue> {
    let proxy = get_autotile_proxy(value);
    if proxy.tilemap == rb_sys::Qnil as VALUE {
        None
    } else {
        Some(get_tilemap(proxy.tilemap))
    }
}

unsafe extern "C" fn autotile_get(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let index = value_to_i32(*argv).max(0) as usize;
    if let Some(tilemap) = proxy_tilemap(self_value) {
        if index < tilemap.autotiles.len() {
            return tilemap.autotiles[index];
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn autotile_set(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = slice::from_raw_parts(argv, 2);
    let index = value_to_i32(args[0]).max(0) as usize;
    if let Some(tilemap) = proxy_tilemap(self_value) {
        set_autotile_value(tilemap, index, args[1]);
        tilemap.autotile_proxy = rb_sys::Qnil as VALUE;
        args[1]
    } else {
        rb_sys::Qnil as VALUE
    }
}

unsafe extern "C" fn autotile_length(
    _argc: c_int,
    _argv: *const VALUE,
    _self_value: VALUE,
) -> VALUE {
    int_to_value(7)
}

unsafe extern "C" fn autotile_each(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    if unsafe { rb_block_given_p() } == 0 {
        return rb_sys::Qnil as VALUE;
    }
    if let Some(tilemap) = proxy_tilemap(self_value) {
        for value in tilemap.autotiles.iter() {
            unsafe {
                rb_yield(*value);
            }
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn autotile_replace(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let array = rb_Array(*argv);
    let len = RARRAY_LEN(array) as usize;
    if let Some(tilemap) = proxy_tilemap(self_value) {
        for index in 0..7 {
            let value = if index < len {
                rb_ary_entry(array, index as c_long)
            } else {
                rb_sys::Qnil as VALUE
            };
            set_autotile_value(tilemap, index, value);
        }
        tilemap.autotile_proxy = rb_sys::Qnil as VALUE;
    }
    *argv
}
