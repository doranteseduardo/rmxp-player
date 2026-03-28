use crate::runtime;
use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rb_sys::{rb_mKernel, rb_obj_class, VALUE};
use std::os::raw::{c_char, c_int};
use tracing::warn;

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
    fn rb_define_method(module: VALUE, name: *const c_char, func: Option<RubyFn>, argc: c_int);
    fn rb_block_given_p() -> c_int;
    fn rb_block_proc() -> VALUE;
}

const RGSS_MAIN_NAME: &[u8] = b"rgss_main\0";
const RGSS_STOP_NAME: &[u8] = b"rgss_stop\0";
const CLASS_NAME: &[u8] = b"class\0";

static KERNEL_FUNCTIONS: OnceCell<()> = OnceCell::new();

pub fn init() -> Result<()> {
    KERNEL_FUNCTIONS
        .get_or_try_init(|| unsafe { define_kernel_functions() })
        .map(|_| ())
}

unsafe fn define_kernel_functions() -> Result<()> {
    let kernel = rb_mKernel;
    if kernel == 0 {
        return Err(anyhow!("rb_mKernel is null"));
    }
    rb_define_module_function(kernel, c_name(RGSS_MAIN_NAME), Some(kernel_rgss_main), -1);
    rb_define_module_function(kernel, c_name(RGSS_STOP_NAME), Some(kernel_rgss_stop), 0);
    rb_define_method(kernel, c_name(CLASS_NAME), Some(kernel_class), -1);
    Ok(())
}

unsafe extern "C" fn kernel_rgss_main(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if rb_block_given_p() == 0 {
        warn!(target: "rgss", "rgss_main called without a block");
        return rb_sys::Qnil as VALUE;
    }
    let block = rb_block_proc();
    if block == 0 {
        warn!(target: "rgss", "Failed to capture rgss_main block");
        return rb_sys::Qnil as VALUE;
    }
    if let Err(err) = runtime::install_main(block) {
        warn!(target: "rgss", error = %err, "Failed to install rgss_main block");
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn kernel_rgss_stop(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Err(err) = runtime::reset_main() {
        warn!(target: "rgss", error = %err, "Failed to reset RGSS runtime");
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn kernel_class(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    rb_obj_class(self_value)
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
