//! Embedded Ruby (MRI) host for RGSS scripts.

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{
    rb_errinfo, rb_eval_string_protect, rb_obj_as_string, rb_string_value_cstr, ruby_init_stack,
    ruby_setup, ruby_sysinit, VALUE,
};
use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    ptr::addr_of_mut,
};
use tracing::{info, warn};

static RUBY_INIT: OnceCell<()> = OnceCell::new();

pub struct RubyVm {
    booted: bool,
}

impl RubyVm {
    pub fn new() -> Self {
        Self { booted: false }
    }

    /// Boots the embedded Ruby interpreter (MRI).
    pub fn boot(&mut self) -> Result<()> {
        ensure_ruby()?;
        if !self.booted {
            info!(target: "rgss", "Ruby VM initialised");
            self.booted = true;
        }
        Ok(())
    }

    /// Evaluate a snippet for diagnostics.
    pub fn eval(&self, code: &str) -> Result<()> {
        ensure_ruby()?;
        let script = CString::new(code)?;
        let mut state: c_int = 0;
        unsafe {
            rb_eval_string_protect(script.as_ptr(), &mut state);
            if state != 0 {
                let message = current_exception_message();
                warn!(target: "rgss", %message, "Ruby eval failed");
                return Err(anyhow!("Ruby eval failed: {message}"));
            }
        }
        Ok(())
    }

    pub fn is_booted(&self) -> bool {
        self.booted
    }
}

fn ensure_ruby() -> Result<()> {
    RUBY_INIT
        .get_or_try_init(|| unsafe { start_ruby() })
        .map(|_| ())
}

unsafe fn start_ruby() -> Result<()> {
    let mut argc: c_int = 0;
    let mut argv: [*mut c_char; 0] = [];
    let mut argv_ptr = argv.as_mut_ptr();
    ruby_sysinit(&mut argc, &mut argv_ptr);

    let mut stack_marker: VALUE = 0;
    ruby_init_stack(addr_of_mut!(stack_marker) as *mut VALUE);

    let code = ruby_setup();
    if code != 0 {
        return Err(anyhow!("ruby_setup failed with code {code}"));
    }
    Ok(())
}

unsafe fn current_exception_message() -> String {
    let err = rb_errinfo();
    let mut string = rb_obj_as_string(err);
    let ptr = rb_string_value_cstr(&mut string);
    CStr::from_ptr(ptr).to_string_lossy().into_owned()
}
