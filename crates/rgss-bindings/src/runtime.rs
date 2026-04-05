use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{rb_cObject, VALUE};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

extern "C" {
    fn rb_const_get(module: VALUE, id: ID) -> VALUE;
    fn rb_intern(name: *const c_char) -> ID;
    fn rb_funcall(recv: VALUE, mid: ID, argc: c_int, ...) -> VALUE;
    fn rb_protect(
        func: Option<unsafe extern "C" fn(VALUE) -> VALUE>,
        arg: VALUE,
        state: *mut c_int,
    ) -> VALUE;
    fn rb_obj_is_kind_of(obj: VALUE, klass: VALUE) -> VALUE;
    fn rb_utf8_str_new(ptr: *const c_char, len: i64) -> VALUE;
    fn rb_gv_set(name: *const c_char, val: VALUE) -> VALUE;
    fn rb_eval_string_protect(str: *const c_char, state: *mut c_int) -> VALUE;
}

type ID = usize;

/// Outcome of a single `resume_main()` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainResult {
    /// The Fiber is still alive; call again next frame.
    Active,
    /// The Fiber returned normally; the game loop is done.
    Done,
    /// A `Reset` exception was raised; caller should re-evaluate scripts.
    Reset,
}

struct RuntimeCache {
    module: VALUE,
    #[allow(dead_code)]
    install_id: ID,
    #[allow(dead_code)]
    install_from_source_id: ID,
    resume_id: ID,
    yield_id: ID,
    active_id: ID,
    pending_events_id: ID,
    #[allow(dead_code)]
    consume_low_memory_id: ID,
    reset_id: ID,
    suspend_notify_id: ID,
    resume_notify_id: ID,
    low_memory_id: ID,
}

static RUNTIME: OnceCell<RuntimeCache> = OnceCell::new();

pub fn is_main_active() -> Result<bool> {
    let cache = runtime_cache()?;
    let value = unsafe { rb_funcall(cache.module, cache.active_id, 0) };
    Ok(value != rb_sys::Qfalse as VALUE)
}

/// Resume the main Fiber by one step.
///
/// Returns:
/// - `Ok(MainResult::Active)` — Fiber yielded normally, call again next frame.
/// - `Ok(MainResult::Done)`   — Fiber finished cleanly.
/// - `Ok(MainResult::Reset)`  — A `Reset` exception was raised; re-run scripts.
/// - `Err(_)`                 — An unexpected Ruby exception occurred.
pub fn resume_main() -> Result<MainResult> {
    let cache = runtime_cache()?;
    let mut state = 0;
    let value = unsafe { rb_protect(Some(call_resume), cache.module, &mut state) };
    if state != 0 {
        // An exception was raised. Check whether it is a Reset.
        if unsafe { is_reset_exception() } {
            // Clear the exception so Ruby remains usable.
            unsafe { rb_sys::rb_set_errinfo(rb_sys::Qnil as VALUE) };
            return Ok(MainResult::Reset);
        }
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Ruby exception during rgss_main: {message}"));
    }
    if value == rb_sys::Qfalse as VALUE || value == rb_sys::Qnil as VALUE {
        Ok(MainResult::Done)
    } else {
        Ok(MainResult::Active)
    }
}

/// Returns true if the current pending exception is a `Reset`.
unsafe fn is_reset_exception() -> bool {
    let err = rb_sys::rb_errinfo();
    if err == rb_sys::Qnil as VALUE {
        return false;
    }
    // Try to resolve the Reset constant; if it doesn't exist the game doesn't use it.
    let reset_name = CString::new("Reset").expect("cstr");
    let reset_id = rb_intern(reset_name.as_ptr());
    let has_reset = rb_funcall(
        rb_cObject,
        {
            let n = CString::new("const_defined?").expect("cstr");
            rb_intern(n.as_ptr())
        },
        1,
        reset_id as VALUE,
    );
    if has_reset == rb_sys::Qfalse as VALUE || has_reset == rb_sys::Qnil as VALUE {
        return false;
    }
    let reset_class = rb_const_get(rb_cObject, reset_id);
    if reset_class == 0 || reset_class == rb_sys::Qnil as VALUE {
        return false;
    }
    rb_obj_is_kind_of(err, reset_class) != rb_sys::Qfalse as VALUE
}

pub fn yield_frame() -> Result<()> {
    let cache = runtime_cache()?;
    let mut state = 0;
    unsafe {
        rb_protect(Some(call_yield), cache.module, &mut state);
    }
    if state != 0 {
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Ruby exception during Graphics.update: {message}"));
    }
    Ok(())
}

pub fn notify_suspend() -> Result<()> {
    runtime_notify("notify_suspend", |cache| cache.suspend_notify_id)
}

pub fn notify_resume() -> Result<()> {
    runtime_notify("notify_resume", |cache| cache.resume_notify_id)
}

pub fn notify_low_memory() -> Result<()> {
    runtime_notify("notify_low_memory", |cache| cache.low_memory_id)
}

#[allow(dead_code)]
pub fn install_main(block: VALUE) -> Result<()> {
    runtime_cache()?;
    let mut state = 0;
    unsafe {
        rb_protect(Some(call_install), block, &mut state);
    }
    if state != 0 {
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Ruby exception during rgss_main: {message}"));
    }
    Ok(())
}

/// Wrap the Main script source string in a Ruby Fiber and store it as the
/// main fiber.  The event loop then drives it via `resume_main()` one frame
/// at a time.  This makes both PE-style synchronous scripts and standard
/// `rgss_main { }` scripts work identically: `Graphics.update` calls
/// `Fiber.yield` inside the fiber and suspends until the next frame.
/// Wrap the Main script source string in a Ruby Fiber and store it as the
/// main fiber.  The event loop then drives it via `resume_main()` one frame
/// at a time.  This makes both PE-style synchronous scripts and standard
/// `rgss_main { }` scripts work identically: `Graphics.update` calls
/// `Fiber.yield` inside the fiber and suspends until the next frame.
pub fn install_main_from_source(source: &str, label: &str) -> Result<()> {
    let _cache = runtime_cache()?;
    // Store source and label in Ruby globals so we can pass them to
    // RGSS::Runtime.install_main_from_source without embedding potentially-large
    // strings in an eval'd bootstrap literal.
    unsafe {
        let src_bytes = source.as_bytes();
        let src_val =
            rb_utf8_str_new(src_bytes.as_ptr() as *const c_char, src_bytes.len() as i64);
        rb_gv_set(b"$__rgss_main_src__\0".as_ptr() as *const c_char, src_val);

        let lbl_bytes = label.as_bytes();
        let lbl_val =
            rb_utf8_str_new(lbl_bytes.as_ptr() as *const c_char, lbl_bytes.len() as i64);
        rb_gv_set(b"$__rgss_main_lbl__\0".as_ptr() as *const c_char, lbl_val);
    }

    let mut state: c_int = 0;
    unsafe {
        let bootstrap =
            b"RGSS::Runtime.install_main_from_source($__rgss_main_src__, $__rgss_main_lbl__); $__rgss_main_src__ = $__rgss_main_lbl__ = nil\0";
        rb_eval_string_protect(bootstrap.as_ptr() as *const c_char, &mut state);
    }
    if state != 0 {
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Failed to install main script fiber: {message}"));
    }
    Ok(())
}

pub fn reset_main() -> Result<()> {
    runtime_notify("rgss_stop", |cache| cache.reset_id)
}

fn runtime_notify<F>(label: &str, id: F) -> Result<()>
where
    F: Fn(&RuntimeCache) -> ID,
{
    let cache = runtime_cache()?;
    let mut state = 0;
    let method_id = id(cache);
    unsafe {
        rb_protect(Some(call_notify), method_id as VALUE, &mut state);
    }
    if state != 0 {
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Ruby exception during {label}: {message}"));
    }
    Ok(())
}

pub fn pending_events() -> Result<bool> {
    runtime_bool("pending_events?", |cache| cache.pending_events_id)
}

#[allow(dead_code)]
pub fn consume_low_memory() -> Result<bool> {
    runtime_bool("consume_low_memory!", |cache| cache.consume_low_memory_id)
}

fn runtime_bool<F>(label: &str, id: F) -> Result<bool>
where
    F: Fn(&RuntimeCache) -> ID,
{
    let cache = runtime_cache()?;
    let mut state = 0;
    let method_id = id(cache);
    let value = unsafe { rb_protect(Some(call_notify), method_id as VALUE, &mut state) };
    if state != 0 {
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Ruby exception during {label}: {message}"));
    }
    Ok(value != rb_sys::Qfalse as VALUE && value != rb_sys::Qnil as VALUE)
}

unsafe extern "C" fn call_resume(arg: VALUE) -> VALUE {
    let cache = RUNTIME.get().expect("runtime cache must be initialised");
    rb_funcall(arg, cache.resume_id, 0)
}

#[allow(dead_code)]
unsafe extern "C" fn call_install(block: VALUE) -> VALUE {
    let cache = RUNTIME.get().expect("runtime cache must be initialised");
    rb_funcall(cache.module, cache.install_id, 1, block)
}

unsafe extern "C" fn call_yield(arg: VALUE) -> VALUE {
    let cache = RUNTIME.get().expect("runtime cache must be initialised");
    rb_funcall(arg, cache.yield_id, 0)
}

unsafe extern "C" fn call_notify(arg: VALUE) -> VALUE {
    let cache = RUNTIME.get().expect("runtime cache must be initialised");
    rb_funcall(cache.module, arg as ID, 0)
}

fn runtime_cache() -> Result<&'static RuntimeCache> {
    RUNTIME.get_or_try_init(|| unsafe {
        let rgss = rb_const_get(rb_cObject, intern("RGSS")?);
        if rgss == 0 {
            return Err(anyhow!("RGSS module not defined"));
        }
        let runtime = rb_const_get(rgss, intern("Runtime")?);
        if runtime == 0 {
            return Err(anyhow!("RGSS::Runtime not defined"));
        }
        Ok(RuntimeCache {
            module: runtime,
            install_id: intern("install_main")?,
            install_from_source_id: intern("install_main_from_source")?,
            resume_id: intern("resume_main")?,
            yield_id: intern("yield_frame")?,
            active_id: intern("active?")?,
            pending_events_id: intern("pending_events?")?,
            consume_low_memory_id: intern("consume_low_memory!")?,
            reset_id: intern("reset")?,
            suspend_notify_id: intern("notify_suspend")?,
            resume_notify_id: intern("notify_resume")?,
            low_memory_id: intern("notify_low_memory")?,
        })
    })
}

unsafe fn intern(name: &str) -> Result<ID> {
    let cstr = CString::new(name)?;
    Ok(rb_intern(cstr.as_ptr()))
}
