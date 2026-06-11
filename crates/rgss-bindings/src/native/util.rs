use rb_sys::stable_api::StableApiDefinition;
use rb_sys::{rb_num2dbl, rb_num2long, rb_str_length, rb_string_value_ptr, ruby_value_type, VALUE};

pub fn value_to_bool(value: VALUE) -> bool {
    value != rb_sys::Qfalse as VALUE && value != rb_sys::Qnil as VALUE
}

/// Ruby type tag for a VALUE (via rb-sys' stable API; cannot raise).
fn rb_value_type(value: VALUE) -> ruby_value_type {
    unsafe { rb_sys::stable_api::get_default().rb_type(value) }
}

/// Read a Ruby String VALUE into a Rust String WITHOUT risking a `longjmp`.
///
/// `rb_string_value_cstr` raises (TypeError on a non-String, ArgumentError on an
/// embedded NUL) — unsafe to let escape an `extern "C"` frame. We first confirm
/// the value is a String (so `rb_string_value_ptr` can't raise), then copy
/// exactly `length` bytes (embedded NULs preserved) with the same UTF-8 →
/// latin1-fallback decoding the rest of the engine uses. Non-String → None.
pub fn value_to_string(value: VALUE) -> Option<String> {
    if rb_value_type(value) != ruby_value_type::RUBY_T_STRING {
        return None;
    }
    unsafe {
        let mut v = value;
        let ptr = rb_string_value_ptr(&mut v) as *const u8;
        if ptr.is_null() {
            return Some(String::new());
        }
        let len = rb_num2long(rb_str_length(value)) as usize;
        let bytes = std::slice::from_raw_parts(ptr, len);
        Some(match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| b as char).collect(),
        })
    }
}

/// Convert a Ruby VALUE to i32 WITHOUT risking a Ruby exception (`longjmp`).
///
/// `rb_num2int` raises (TypeError on non-numeric, RangeError on overflow), and a
/// `longjmp` across these `extern "C"` frames — which hold `Drop` Rust state — is
/// undefined behavior. So we only convert values that cannot raise:
///   * Fixnum -> `rb_num2long` (a Fixnum always fits a C long, never raises),
///              truncated to i32 (matches RGSS coordinate/size semantics).
///   * Float  -> `rb_num2dbl` (never raises on a Float), truncated to i32.
/// Anything else (nil, String, Array, Bignum, custom objects) degrades to 0
/// instead of crashing.
pub fn value_to_i32(value: VALUE) -> i32 {
    match rb_value_type(value) {
        ruby_value_type::RUBY_T_FIXNUM => unsafe { rb_num2long(value) as i32 },
        ruby_value_type::RUBY_T_FLOAT => unsafe { rb_num2dbl(value) as i32 },
        _ => 0,
    }
}

/// Convert a Ruby VALUE to f32 WITHOUT risking a Ruby exception (`longjmp`).
/// Only Fixnum/Float are converted; anything else degrades to 0.0.
pub fn value_to_f32(value: VALUE) -> f32 {
    match rb_value_type(value) {
        ruby_value_type::RUBY_T_FIXNUM => unsafe { rb_num2long(value) as f32 },
        ruby_value_type::RUBY_T_FLOAT => unsafe { rb_num2dbl(value) as f32 },
        _ => 0.0,
    }
}
