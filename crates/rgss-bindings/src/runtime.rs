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
}

type ID = usize;

struct RuntimeCache {
    module: VALUE,
    resume_id: ID,
    yield_id: ID,
    active_id: ID,
}

static RUNTIME: OnceCell<RuntimeCache> = OnceCell::new();

pub fn is_main_active() -> Result<bool> {
    let cache = runtime_cache()?;
    let value = unsafe { rb_funcall(cache.module, cache.active_id, 0) };
    Ok(value != rb_sys::Qfalse as VALUE)
}

pub fn resume_main() -> Result<bool> {
    let cache = runtime_cache()?;
    let mut state = 0;
    let value = unsafe { rb_protect(Some(call_resume), cache.module, &mut state) };
    if state != 0 {
        let message = unsafe { crate::current_exception_message() };
        return Err(anyhow!("Ruby exception during rgss_main: {message}"));
    }
    Ok(value != rb_sys::Qfalse as VALUE)
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

unsafe extern "C" fn call_resume(arg: VALUE) -> VALUE {
    let cache = RUNTIME.get().expect("runtime cache must be initialised");
    rb_funcall(arg, cache.resume_id, 0)
}

unsafe extern "C" fn call_yield(arg: VALUE) -> VALUE {
    let cache = RUNTIME.get().expect("runtime cache must be initialised");
    rb_funcall(arg, cache.yield_id, 0)
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
            resume_id: intern("resume_main")?,
            yield_id: intern("yield_frame")?,
            active_id: intern("active?")?,
        })
    })
}

unsafe fn intern(name: &str) -> Result<ID> {
    let cstr = CString::new(name)?;
    Ok(rb_intern(cstr.as_ptr()))
}
