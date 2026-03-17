use rb_sys::{rb_num2dbl, rb_num2int, VALUE};
use std::convert::TryInto;

pub fn value_to_bool(value: VALUE) -> bool {
    value != rb_sys::Qfalse as VALUE && value != rb_sys::Qnil as VALUE
}

pub fn value_to_i32(value: VALUE) -> i32 {
    unsafe { rb_num2int(value).try_into().unwrap() }
}

pub fn value_to_f32(value: VALUE) -> f32 {
    unsafe { rb_num2dbl(value) as f32 }
}
