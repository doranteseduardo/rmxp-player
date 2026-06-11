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

// ── Numeric coercion ────────────────────────────────────────────────────────
//
// `rb_num2int`/`rb_num2uint`/`rb_num2long`/`rb_num2dbl` RAISE a Ruby exception
// (TypeError on a non-numeric value, RangeError on overflow). Letting that
// `longjmp` unwind an `extern "C"` frame that holds `Drop` Rust state is
// undefined behavior. These helpers type-check first and only call a conversion
// that cannot raise for the confirmed type:
//   * Fixnum -> `rb_num2long` (a Fixnum always fits a C long; never raises)
//   * Float  -> `rb_num2dbl` (never raises on a Float)
// Anything else (nil, String, Array, Bignum, custom objects) degrades to 0
// instead of crashing. Bignum is treated as out-of-range -> 0; no RGSS value
// legitimately exceeds i64.

/// Convert a Ruby VALUE to i64 without risking a Ruby exception.
pub fn value_to_i64(value: VALUE) -> i64 {
    match rb_value_type(value) {
        ruby_value_type::RUBY_T_FIXNUM => unsafe { rb_num2long(value) },
        ruby_value_type::RUBY_T_FLOAT => unsafe { rb_num2dbl(value) as i64 },
        _ => 0,
    }
}

/// Convert a Ruby VALUE to f64 without risking a Ruby exception.
pub fn value_to_f64(value: VALUE) -> f64 {
    match rb_value_type(value) {
        ruby_value_type::RUBY_T_FIXNUM => unsafe { rb_num2long(value) as f64 },
        ruby_value_type::RUBY_T_FLOAT => unsafe { rb_num2dbl(value) },
        _ => 0.0,
    }
}

/// Convert a Ruby VALUE to i32 without risking a Ruby exception (truncating).
pub fn value_to_i32(value: VALUE) -> i32 {
    value_to_i64(value) as i32
}

/// Convert a Ruby VALUE to u32 without risking a Ruby exception (low 32 bits).
/// Used for native handles and packed RGBA values, which are non-negative.
pub fn value_to_u32(value: VALUE) -> u32 {
    value_to_i64(value) as u32
}

/// Convert a Ruby VALUE to f32 without risking a Ruby exception (truncating).
pub fn value_to_f32(value: VALUE) -> f32 {
    value_to_f64(value) as f32
}
