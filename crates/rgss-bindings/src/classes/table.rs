use super::common::{
    define_method, get_typed_data, install_allocator, int_to_value, to_c_long, wrap_typed_data,
    DataTypeBuilder, StaticDataType,
};
use crate::native::value_to_i32;
use anyhow::Result;
use once_cell::sync::Lazy;
use rb_sys::{
    bindings::{rb_obj_class, rb_str_new},
    VALUE,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::{c_char, c_int},
    slice,
};

const TABLE_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Table\0") };
const TABLE_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Table\0") };

static TABLE_TYPE: StaticDataType =
    StaticDataType::new(|| DataTypeBuilder::new(TABLE_STRUCT_NAME).free(table_free));

static METHOD_INITIALIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"initialize\0") });
static METHOD_RESIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"resize\0") });
static METHOD_GET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"[]\0") });
static METHOD_SET: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"[]=\0") });
static METHOD_XSIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"xsize\0") });
static METHOD_YSIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"ysize\0") });
static METHOD_ZSIZE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"zsize\0") });
static METHOD_CLONE: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"clone\0") });
static METHOD_DUP: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"dup\0") });
static METHOD_PACK: Lazy<&'static CStr> =
    Lazy::new(|| unsafe { CStr::from_bytes_with_nul_unchecked(b"to_native_s16\0") });

#[derive(Clone)]
struct TableValue {
    xsize: i32,
    ysize: i32,
    zsize: i32,
    data: Vec<i16>,
}

impl Default for TableValue {
    fn default() -> Self {
        Self {
            xsize: 0,
            ysize: 1,
            zsize: 1,
            data: Vec::new(),
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(TABLE_CLASS_NAME, None);
        install_allocator(klass, Some(table_allocate));
        define_method(klass, *METHOD_INITIALIZE, table_initialize, -1);
        define_method(klass, *METHOD_RESIZE, table_resize, -1);
        define_method(klass, *METHOD_GET, table_get, -1);
        define_method(klass, *METHOD_SET, table_set, -1);
        define_method(klass, *METHOD_XSIZE, table_xsize, 0);
        define_method(klass, *METHOD_YSIZE, table_ysize, 0);
        define_method(klass, *METHOD_ZSIZE, table_zsize, 0);
        define_method(klass, *METHOD_CLONE, table_clone, 0);
        define_method(klass, *METHOD_DUP, table_dup, 0);
        define_method(klass, *METHOD_PACK, table_pack, 0);
    }
    Ok(())
}

unsafe extern "C" fn table_allocate(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, TableValue::default(), TABLE_TYPE.as_rb_type())
}

unsafe extern "C" fn table_free(ptr: *mut c_void) {
    drop(Box::<TableValue>::from_raw(ptr as *mut TableValue));
}

fn get_table_mut(value: VALUE) -> &'static mut TableValue {
    unsafe { get_typed_data(value, TABLE_TYPE.as_rb_type()) }.expect("Table missing native data")
}

unsafe extern "C" fn table_initialize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    resize_table(self_value, argc, argv, true);
    self_value
}

unsafe extern "C" fn table_resize(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    resize_table(self_value, argc, argv, false);
    self_value
}

unsafe fn resize_table(obj: VALUE, argc: c_int, argv: *const VALUE, initializing: bool) {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let table = get_table_mut(obj);
    let default_x = if initializing { 0 } else { table.xsize };
    let default_y = if initializing { 1 } else { table.ysize.max(1) };
    let default_z = if initializing { 1 } else { table.zsize.max(1) };
    table.xsize = args
        .get(0)
        .map(|&v| value_to_i32(v).max(0))
        .unwrap_or(default_x);
    table.ysize = args
        .get(1)
        .map(|&v| value_to_i32(v).max(1))
        .unwrap_or(default_y);
    table.zsize = args
        .get(2)
        .map(|&v| value_to_i32(v).max(1))
        .unwrap_or(default_z);
    let total = (table.xsize as usize)
        .saturating_mul(table.ysize as usize)
        .saturating_mul(table.zsize as usize);
    table.data.resize(total, 0);
}

unsafe extern "C" fn table_get(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let x = args.get(0).map(|v| value_to_i32(*v)).unwrap_or(0);
    let y = args.get(1).map(|v| value_to_i32(*v)).unwrap_or(0);
    let z = args.get(2).map(|v| value_to_i32(*v)).unwrap_or(0);
    let table = get_table_mut(self_value);
    if let Some(idx) = index_of(table, x, y, z) {
        return int_to_value(table.data[idx] as i64);
    }
    int_to_value(0)
}

unsafe extern "C" fn table_set(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() || argc < 4 {
        return rb_sys::Qnil as VALUE;
    }
    let args = slice::from_raw_parts(argv, argc as usize);
    let x = value_to_i32(args[0]);
    let y = value_to_i32(args[1]);
    let z = value_to_i32(args[2]);
    let value = clamp_to_i16(value_to_i32(args[3]));
    let table = get_table_mut(self_value);
    if let Some(idx) = index_of(table, x, y, z) {
        table.data[idx] = value;
    }
    args[3]
}

unsafe extern "C" fn table_xsize(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    int_to_value(get_table_mut(self_value).xsize as i64)
}

unsafe extern "C" fn table_ysize(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    int_to_value(get_table_mut(self_value).ysize as i64)
}

unsafe extern "C" fn table_zsize(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    int_to_value(get_table_mut(self_value).zsize as i64)
}

unsafe extern "C" fn table_clone(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let klass = rb_obj_class(self_value);
    let new_obj = table_allocate(klass);
    let source = get_table_mut(self_value).clone();
    *get_table_mut(new_obj) = source;
    new_obj
}

unsafe extern "C" fn table_dup(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    table_clone(argc, argv, self_value)
}

unsafe extern "C" fn table_pack(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let table = get_table_mut(self_value);
    let mut bytes = Vec::with_capacity(table.data.len() * 2);
    for value in &table.data {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    rb_str_new(bytes.as_ptr() as *const c_char, to_c_long(bytes.len()))
}

fn index_of(table: &TableValue, x: i32, y: i32, z: i32) -> Option<usize> {
    if x < 0 || y < 0 || z < 0 {
        return None;
    }
    if x >= table.xsize || y >= table.ysize || z >= table.zsize {
        return None;
    }
    let idx = x as usize
        + y as usize * table.xsize as usize
        + z as usize * table.xsize as usize * table.ysize as usize;
    Some(idx)
}

#[derive(Clone)]
pub struct TableSnapshot {
    pub xsize: i32,
    pub ysize: i32,
    pub zsize: i32,
    pub data: Vec<i16>,
}

pub fn table_snapshot(value: VALUE) -> Option<TableSnapshot> {
    unsafe { get_typed_data::<TableValue>(value, TABLE_TYPE.as_rb_type()) }.map(|table| {
        TableSnapshot {
            xsize: table.xsize,
            ysize: table.ysize,
            zsize: table.zsize,
            data: table.data.clone(),
        }
    })
}

fn clamp_to_i16(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}
