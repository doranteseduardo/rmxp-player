use crate::native::module::native_module;
use anyhow::Result;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rb_sys::{rb_num2int, VALUE};
use std::{collections::VecDeque, os::raw::c_char};

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<unsafe extern "C" fn(std::os::raw::c_int, *const VALUE, VALUE) -> VALUE>,
        argc: std::os::raw::c_int,
    );
}

const REQUEST_PAUSE_NAME: &[u8] = b"interpreter_request_pause\0";
const REQUEST_MAP_RELOAD_NAME: &[u8] = b"interpreter_request_map_reload\0";

static COMMAND_QUEUE: Lazy<Mutex<VecDeque<InterpreterCommand>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpreterCommand {
    Pause,
    ReloadMap { map_id: i32 },
}

pub fn init() -> Result<()> {
    let module = native_module()?;
    unsafe {
        rb_define_module_function(
            module,
            c_name(REQUEST_PAUSE_NAME),
            Some(interpreter_request_pause),
            0,
        );
        rb_define_module_function(
            module,
            c_name(REQUEST_MAP_RELOAD_NAME),
            Some(interpreter_request_map_reload),
            1,
        );
    }
    Ok(())
}

pub fn enqueue(command: InterpreterCommand) {
    COMMAND_QUEUE.lock().push_back(command);
}

pub fn drain_commands() -> Vec<InterpreterCommand> {
    COMMAND_QUEUE.lock().drain(..).collect()
}

unsafe extern "C" fn interpreter_request_pause(
    _argc: std::os::raw::c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    enqueue(InterpreterCommand::Pause);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn interpreter_request_map_reload(
    argc: std::os::raw::c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc <= 0 || argv.is_null() {
        warn_unexpected_arity("interpreter_request_map_reload", argc);
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let map_id = rb_num2int(args[0]) as i32;
    enqueue(InterpreterCommand::ReloadMap { map_id });
    rb_sys::Qnil as VALUE
}

fn warn_unexpected_arity(label: &str, argc: std::os::raw::c_int) {
    tracing::warn!(
        target: "rgss",
        function = %label,
        argc,
        "interpreter bridge called with unexpected arity"
    );
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
