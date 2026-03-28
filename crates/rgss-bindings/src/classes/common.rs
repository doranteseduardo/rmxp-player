#![allow(dead_code)]

use once_cell::sync::OnceCell;
use rb_sys::{
    bindings::{rb_float_new, rb_ll2inum, rb_str_new, rb_utf8_str_new},
    macros::RTYPEDDATA_GET_DATA,
    rb_alloc_func_t, rb_cObject, rb_data_type_t, rb_data_typed_object_wrap, rb_define_alloc_func,
    rb_define_class, rb_typeddata_is_kind_of, size_t, VALUE,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::{c_char, c_int, c_long, c_longlong},
    ptr,
};

pub type RubyMethod = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_method(klass: VALUE, name: *const c_char, func: Option<RubyMethod>, argc: c_int);
    fn rb_define_singleton_method(
        klass: VALUE,
        name: *const c_char,
        func: Option<RubyMethod>,
        argc: c_int,
    );
    pub fn rb_string_value_ptr(v: *mut VALUE) -> *const c_char;
    pub fn rb_str_length(v: VALUE) -> VALUE;
    pub fn rb_num2long(v: VALUE) -> c_long;
}

#[repr(C)]
struct NativeDataTypeFunction {
    dmark: Option<unsafe extern "C" fn(*mut c_void)>,
    dfree: Option<unsafe extern "C" fn(*mut c_void)>,
    dsize: Option<unsafe extern "C" fn(*const c_void) -> size_t>,
    #[cfg(ruby_have_rb_data_type_t_function)]
    dcompact: Option<unsafe extern "C" fn(*mut c_void)>,
    #[cfg(ruby_have_rb_data_type_t_function)]
    reserved: [*mut c_void; 1],
    #[cfg(not(ruby_have_rb_data_type_t_function))]
    reserved: [*mut c_void; 2],
}

#[repr(C)]
struct NativeDataType {
    wrap_struct_name: *const i8,
    function: NativeDataTypeFunction,
    parent: *const rb_data_type_t,
    data: *mut c_void,
    flags: VALUE,
}

unsafe impl Send for NativeDataType {}
unsafe impl Sync for NativeDataType {}

unsafe impl Send for NativeDataTypeFunction {}
unsafe impl Sync for NativeDataTypeFunction {}

pub struct DataTypeBuilder {
    name: &'static CStr,
    mark: Option<unsafe extern "C" fn(*mut c_void)>,
    free: Option<unsafe extern "C" fn(*mut c_void)>,
    size: Option<unsafe extern "C" fn(*const c_void) -> size_t>,
    compact: Option<unsafe extern "C" fn(*mut c_void)>,
    flags: VALUE,
}

impl DataTypeBuilder {
    pub const fn new(name: &'static CStr) -> Self {
        Self {
            name,
            mark: None,
            free: None,
            size: None,
            compact: None,
            flags: 0,
        }
    }

    pub const fn mark(mut self, func: unsafe extern "C" fn(*mut c_void)) -> Self {
        self.mark = Some(func);
        self
    }

    pub const fn free(mut self, func: unsafe extern "C" fn(*mut c_void)) -> Self {
        self.free = Some(func);
        self
    }

    pub const fn size(mut self, func: unsafe extern "C" fn(*const c_void) -> size_t) -> Self {
        self.size = Some(func);
        self
    }

    pub const fn compact(mut self, func: unsafe extern "C" fn(*mut c_void)) -> Self {
        self.compact = Some(func);
        self
    }

    pub const fn flags(mut self, flags: VALUE) -> Self {
        self.flags = flags;
        self
    }

    fn build(self) -> NativeDataType {
        NativeDataType {
            wrap_struct_name: self.name.as_ptr(),
            function: NativeDataTypeFunction {
                dmark: self.mark,
                dfree: self.free,
                dsize: self.size,
                #[cfg(ruby_have_rb_data_type_t_function)]
                dcompact: self.compact,
                reserved: [ptr::null_mut(); reserved_len()],
            },
            parent: ptr::null(),
            data: ptr::null_mut(),
            flags: self.flags,
        }
    }
}

#[cfg(ruby_have_rb_data_type_t_function)]
const fn reserved_len() -> usize {
    1
}

#[cfg(not(ruby_have_rb_data_type_t_function))]
const fn reserved_len() -> usize {
    2
}

pub struct StaticDataType {
    builder: fn() -> DataTypeBuilder,
    cell: OnceCell<NativeDataType>,
}

impl StaticDataType {
    pub const fn new(builder: fn() -> DataTypeBuilder) -> Self {
        Self {
            builder,
            cell: OnceCell::new(),
        }
    }

    pub fn as_rb_type(&self) -> &'static rb_data_type_t {
        let value = self.cell.get_or_init(|| (self.builder)().build());
        unsafe { &*(value as *const NativeDataType as *const rb_data_type_t) }
    }
}

pub unsafe fn wrap_typed_data<T>(
    klass: VALUE,
    data: T,
    data_type: &'static rb_data_type_t,
) -> VALUE {
    let boxed = Box::new(data);
    rb_data_typed_object_wrap(klass, Box::into_raw(boxed) as *mut c_void, data_type)
}

pub unsafe fn get_typed_data<T>(
    value: VALUE,
    data_type: &'static rb_data_type_t,
) -> Option<&'static mut T> {
    if rb_typeddata_is_kind_of(value, data_type) == 0 {
        return None;
    }
    let ptr = RTYPEDDATA_GET_DATA(value) as *mut T;
    ptr.as_mut()
}

pub fn define_ruby_class(name: &'static CStr, parent: Option<VALUE>) -> VALUE {
    let parent = parent.unwrap_or(unsafe { rb_cObject });
    unsafe { rb_define_class(name.as_ptr(), parent) }
}

pub fn install_allocator(klass: VALUE, func: rb_alloc_func_t) {
    unsafe { rb_define_alloc_func(klass, func) }
}

pub unsafe fn define_method(klass: VALUE, name: &'static CStr, func: RubyMethod, argc: c_int) {
    rb_define_method(klass, name.as_ptr(), Some(func), argc);
}

pub unsafe fn define_singleton_method(
    klass: VALUE,
    name: &'static CStr,
    func: RubyMethod,
    argc: c_int,
) {
    rb_define_singleton_method(klass, name.as_ptr(), Some(func), argc);
}

pub fn bool_to_value(value: bool) -> VALUE {
    if value {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

pub fn int_to_value(value: i64) -> VALUE {
    unsafe { rb_ll2inum(value as c_longlong) }
}

pub fn float_to_value(value: f64) -> VALUE {
    unsafe { rb_float_new(value) }
}

pub fn bytes_to_str(bytes: &[u8]) -> VALUE {
    unsafe { rb_str_new(bytes.as_ptr() as *const c_char, to_c_long(bytes.len())) }
}

pub fn utf8_str(text: &str) -> VALUE {
    unsafe { rb_utf8_str_new(text.as_ptr() as *const c_char, to_c_long(text.len())) }
}

pub fn to_c_long(len: usize) -> c_long {
    len.try_into().expect("length exceeds c_long range")
}

/// Get the raw bytes from a Ruby String VALUE (safe for binary data with null bytes).
pub unsafe fn ruby_string_bytes(val: VALUE) -> Option<Vec<u8>> {
    let mut v = val;
    let ptr = rb_string_value_ptr(&mut v);
    if ptr.is_null() {
        return None;
    }
    let len = rb_num2long(rb_str_length(val)) as usize;
    Some(std::slice::from_raw_parts(ptr as *const u8, len).to_vec())
}
