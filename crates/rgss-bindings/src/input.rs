use crate::{native::value_to_bool, system::current_delta};
use anyhow::{anyhow, Result};
use arboard::Clipboard;
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    rb_define_module, rb_float_new, rb_id2sym, rb_int2big, rb_intern, rb_num2int,
    rb_string_value_cstr, rb_utf8_str_new, special_consts, VALUE,
};
use std::{
    ffi::CString,
    os::raw::{c_char, c_int},
    sync::{Mutex, RwLock},
};

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
    fn rb_define_const(module: VALUE, name: *const c_char, value: VALUE);
    fn rb_define_module_under(outer: VALUE, name: *const c_char) -> VALUE;
}

const INPUT_NAME: &[u8] = b"Input\0";
const CONTROLLER_NAME: &[u8] = b"Controller\0";

const DELTA_NAME: &[u8] = b"delta\0";
const UPDATE_NAME: &[u8] = b"update\0";
const PRESS_Q_NAME: &[u8] = b"press?\0";
const TRIGGER_Q_NAME: &[u8] = b"trigger?\0";
const REPEAT_Q_NAME: &[u8] = b"repeat?\0";
const RELEASE_Q_NAME: &[u8] = b"release?\0";
const COUNT_NAME: &[u8] = b"count\0";
const TIME_Q_NAME: &[u8] = b"time?\0";
const DIR4_NAME: &[u8] = b"dir4\0";
const DIR8_NAME: &[u8] = b"dir8\0";
const MOUSE_X_NAME: &[u8] = b"mouse_x\0";
const MOUSE_Y_NAME: &[u8] = b"mouse_y\0";
const SCROLL_V_NAME: &[u8] = b"scroll_v\0";
const MOUSE_IN_WINDOW_NAME: &[u8] = b"mouse_in_window\0";
const MOUSE_IN_WINDOW_Q_NAME: &[u8] = b"mouse_in_window?\0";
const RAW_KEY_STATES_NAME: &[u8] = b"raw_key_states\0";
const TEXT_INPUT_NAME: &[u8] = b"text_input\0";
const TEXT_INPUT_SET_NAME: &[u8] = b"text_input=\0";
const GETS_NAME: &[u8] = b"gets\0";
const CLIPBOARD_NAME: &[u8] = b"clipboard\0";
const CLIPBOARD_SET_NAME: &[u8] = b"clipboard=\0";

const CONTROLLER_CONNECTED_Q_NAME: &[u8] = b"connected?\0";
const CONTROLLER_NAME_NAME: &[u8] = b"name\0";
const CONTROLLER_POWER_NAME: &[u8] = b"power_level\0";
const CONTROLLER_AXES_LEFT_NAME: &[u8] = b"axes_left\0";
const CONTROLLER_AXES_RIGHT_NAME: &[u8] = b"axes_right\0";
const CONTROLLER_AXES_TRIGGER_NAME: &[u8] = b"axes_trigger\0";
const CONTROLLER_RAW_BUTTONS_NAME: &[u8] = b"raw_button_states\0";
const CONTROLLER_RAW_AXES_NAME: &[u8] = b"raw_axes\0";
const CONTROLLER_PRESS_EX_NAME: &[u8] = b"pressex?\0";
const CONTROLLER_TRIGGER_EX_NAME: &[u8] = b"triggerex?\0";
const CONTROLLER_REPEAT_EX_NAME: &[u8] = b"repeatex?\0";
const CONTROLLER_RELEASE_EX_NAME: &[u8] = b"releaseex?\0";
const CONTROLLER_REPEATCOUNT_NAME: &[u8] = b"repeatcount\0";
const CONTROLLER_TIME_EX_NAME: &[u8] = b"timeex?\0";

const REPEAT_DELAY: u16 = 20;
const REPEAT_INTERVAL: u16 = 5;

type ButtonMask = u32;
const BUTTON_COUNT: usize = 25;

#[derive(Clone, Copy)]
struct ButtonConst {
    name: &'static [u8],
    rgss_id: i32,
    mask: ButtonMask,
}

const BUTTON_TABLE: [ButtonConst; BUTTON_COUNT] = [
    ButtonConst {
        name: b"DOWN\0",
        rgss_id: 2,
        mask: BUTTON_DOWN,
    },
    ButtonConst {
        name: b"LEFT\0",
        rgss_id: 4,
        mask: BUTTON_LEFT,
    },
    ButtonConst {
        name: b"RIGHT\0",
        rgss_id: 6,
        mask: BUTTON_RIGHT,
    },
    ButtonConst {
        name: b"UP\0",
        rgss_id: 8,
        mask: BUTTON_UP,
    },
    ButtonConst {
        name: b"A\0",
        rgss_id: 11,
        mask: BUTTON_A,
    },
    ButtonConst {
        name: b"B\0",
        rgss_id: 12,
        mask: BUTTON_B,
    },
    ButtonConst {
        name: b"C\0",
        rgss_id: 13,
        mask: BUTTON_C,
    },
    ButtonConst {
        name: b"X\0",
        rgss_id: 14,
        mask: BUTTON_X,
    },
    ButtonConst {
        name: b"Y\0",
        rgss_id: 15,
        mask: BUTTON_Y,
    },
    ButtonConst {
        name: b"Z\0",
        rgss_id: 16,
        mask: BUTTON_Z,
    },
    ButtonConst {
        name: b"L\0",
        rgss_id: 17,
        mask: BUTTON_L,
    },
    ButtonConst {
        name: b"R\0",
        rgss_id: 18,
        mask: BUTTON_R,
    },
    ButtonConst {
        name: b"SHIFT\0",
        rgss_id: 21,
        mask: BUTTON_SHIFT,
    },
    ButtonConst {
        name: b"CTRL\0",
        rgss_id: 22,
        mask: BUTTON_CTRL,
    },
    ButtonConst {
        name: b"ALT\0",
        rgss_id: 23,
        mask: BUTTON_ALT,
    },
    ButtonConst {
        name: b"F5\0",
        rgss_id: 25,
        mask: BUTTON_F5,
    },
    ButtonConst {
        name: b"F6\0",
        rgss_id: 26,
        mask: BUTTON_F6,
    },
    ButtonConst {
        name: b"F7\0",
        rgss_id: 27,
        mask: BUTTON_F7,
    },
    ButtonConst {
        name: b"F8\0",
        rgss_id: 28,
        mask: BUTTON_F8,
    },
    ButtonConst {
        name: b"F9\0",
        rgss_id: 29,
        mask: BUTTON_F9,
    },
    ButtonConst {
        name: b"MOUSELEFT\0",
        rgss_id: 38,
        mask: BUTTON_MOUSE_LEFT,
    },
    ButtonConst {
        name: b"MOUSEMIDDLE\0",
        rgss_id: 39,
        mask: BUTTON_MOUSE_MIDDLE,
    },
    ButtonConst {
        name: b"MOUSERIGHT\0",
        rgss_id: 40,
        mask: BUTTON_MOUSE_RIGHT,
    },
    ButtonConst {
        name: b"MOUSEX1\0",
        rgss_id: 41,
        mask: BUTTON_MOUSE_X1,
    },
    ButtonConst {
        name: b"MOUSEX2\0",
        rgss_id: 42,
        mask: BUTTON_MOUSE_X2,
    },
];

static INPUT_MODULE: OnceCell<()> = OnceCell::new();
static STORE: Lazy<Mutex<InputStore>> = Lazy::new(|| Mutex::new(InputStore::default()));
static MOUSE_STATE: Lazy<RwLock<MouseState>> = Lazy::new(|| RwLock::new(MouseState::default()));
static TEXT_STATE: Lazy<RwLock<TextState>> = Lazy::new(|| RwLock::new(TextState::default()));

pub const BUTTON_DOWN: ButtonMask = 1 << 0;
pub const BUTTON_LEFT: ButtonMask = 1 << 1;
pub const BUTTON_RIGHT: ButtonMask = 1 << 2;
pub const BUTTON_UP: ButtonMask = 1 << 3;
pub const BUTTON_A: ButtonMask = 1 << 4;
pub const BUTTON_B: ButtonMask = 1 << 5;
pub const BUTTON_C: ButtonMask = 1 << 6;
pub const BUTTON_X: ButtonMask = 1 << 7;
pub const BUTTON_Y: ButtonMask = 1 << 8;
pub const BUTTON_Z: ButtonMask = 1 << 9;
pub const BUTTON_L: ButtonMask = 1 << 10;
pub const BUTTON_R: ButtonMask = 1 << 11;
pub const BUTTON_SHIFT: ButtonMask = 1 << 12;
pub const BUTTON_CTRL: ButtonMask = 1 << 13;
pub const BUTTON_ALT: ButtonMask = 1 << 14;
pub const BUTTON_F5: ButtonMask = 1 << 15;
pub const BUTTON_F6: ButtonMask = 1 << 16;
pub const BUTTON_F7: ButtonMask = 1 << 17;
pub const BUTTON_F8: ButtonMask = 1 << 18;
pub const BUTTON_F9: ButtonMask = 1 << 19;
pub const BUTTON_MOUSE_LEFT: ButtonMask = 1 << 20;
pub const BUTTON_MOUSE_MIDDLE: ButtonMask = 1 << 21;
pub const BUTTON_MOUSE_RIGHT: ButtonMask = 1 << 22;
pub const BUTTON_MOUSE_X1: ButtonMask = 1 << 23;
pub const BUTTON_MOUSE_X2: ButtonMask = 1 << 24;

#[derive(Debug, Clone)]
pub enum TextEvent {
    Insert(char),
    Backspace,
}

#[derive(Debug, Clone)]
pub struct InputSnapshot {
    mask: ButtonMask,
    mouse_position: Option<(f32, f32)>,
    mouse_in_window: bool,
    scroll_v: f32,
    text_events: Vec<TextEvent>,
}

impl Default for InputSnapshot {
    fn default() -> Self {
        Self {
            mask: 0,
            mouse_position: None,
            mouse_in_window: false,
            scroll_v: 0.0,
            text_events: Vec::new(),
        }
    }
}

impl InputSnapshot {
    pub fn set_mask(&mut self, mask: ButtonMask) {
        self.mask = mask;
    }

    pub fn with_button(mut self, button: ButtonMask, pressed: bool) -> Self {
        self.set_button(button, pressed);
        self
    }

    pub fn set_button(&mut self, button: ButtonMask, pressed: bool) {
        if pressed {
            self.mask |= button;
        } else {
            self.mask &= !button;
        }
    }

    pub fn set_mouse(&mut self, position: Option<(f32, f32)>, in_window: bool) {
        self.mouse_position = position;
        self.mouse_in_window = in_window;
    }

    pub fn set_scroll(&mut self, scroll: f32) {
        self.scroll_v = scroll;
    }

    pub fn push_text_event(&mut self, event: TextEvent) {
        self.text_events.push(event);
    }

    pub fn mask(&self) -> ButtonMask {
        self.mask
    }
}

pub fn init() -> Result<()> {
    INPUT_MODULE
        .get_or_try_init(|| unsafe { define_input() })
        .map(|_| ())
}

pub fn update_input(snapshot: InputSnapshot) {
    let InputSnapshot {
        mask,
        mouse_position,
        mouse_in_window,
        scroll_v,
        text_events,
    } = snapshot;
    if let Ok(mut store) = STORE.lock() {
        store.ingest(mask);
    }
    if let Ok(mut mouse) = MOUSE_STATE.write() {
        mouse.position = mouse_position;
        mouse.in_window = mouse_in_window;
        mouse.scroll_v = scroll_v;
    }
    if let Ok(mut text) = TEXT_STATE.write() {
        if text.enabled {
            for event in text_events {
                match event {
                    TextEvent::Insert(ch) => text.buffer.push(ch),
                    TextEvent::Backspace => {
                        text.buffer.pop();
                    }
                }
            }
        }
    }
}

unsafe fn define_input() -> Result<()> {
    let module = rb_define_module(c_name(INPUT_NAME));
    if module == 0 {
        return Err(anyhow!("failed to define Input module"));
    }

    rb_define_module_function(module, c_name(DELTA_NAME), Some(input_delta), 0);
    rb_define_module_function(module, c_name(UPDATE_NAME), Some(input_update), -1);
    rb_define_module_function(module, c_name(PRESS_Q_NAME), Some(input_press_qmark), 1);
    rb_define_module_function(module, c_name(TRIGGER_Q_NAME), Some(input_trigger_qmark), 1);
    rb_define_module_function(module, c_name(REPEAT_Q_NAME), Some(input_repeat_qmark), 1);
    rb_define_module_function(module, c_name(RELEASE_Q_NAME), Some(input_release_qmark), 1);
    rb_define_module_function(module, c_name(COUNT_NAME), Some(input_count), 1);
    rb_define_module_function(module, c_name(TIME_Q_NAME), Some(input_time_qmark), 1);
    rb_define_module_function(module, c_name(DIR4_NAME), Some(input_dir4), 0);
    rb_define_module_function(module, c_name(DIR8_NAME), Some(input_dir8), 0);
    rb_define_module_function(module, c_name(MOUSE_X_NAME), Some(input_mouse_x), 0);
    rb_define_module_function(module, c_name(MOUSE_Y_NAME), Some(input_mouse_y), 0);
    rb_define_module_function(module, c_name(SCROLL_V_NAME), Some(input_scroll_v), 0);
    rb_define_module_function(
        module,
        c_name(MOUSE_IN_WINDOW_NAME),
        Some(input_mouse_in_window_q),
        0,
    );
    rb_define_module_function(
        module,
        c_name(MOUSE_IN_WINDOW_Q_NAME),
        Some(input_mouse_in_window_q),
        0,
    );
    rb_define_module_function(
        module,
        c_name(RAW_KEY_STATES_NAME),
        Some(input_raw_key_states),
        0,
    );
    rb_define_module_function(module, c_name(TEXT_INPUT_NAME), Some(input_text_input), 0);
    rb_define_module_function(
        module,
        c_name(TEXT_INPUT_SET_NAME),
        Some(input_set_text_input),
        1,
    );
    rb_define_module_function(module, c_name(GETS_NAME), Some(input_gets), 0);
    rb_define_module_function(module, c_name(CLIPBOARD_NAME), Some(input_clipboard), 0);
    rb_define_module_function(
        module,
        c_name(CLIPBOARD_SET_NAME),
        Some(input_set_clipboard),
        1,
    );

    define_controller_module(module);

    for entry in BUTTON_TABLE {
        rb_define_const(
            module,
            c_name(entry.name),
            int_to_value(entry.rgss_id as i64),
        );
    }
    Ok(())
}

unsafe fn define_controller_module(parent: VALUE) {
    let controller = rb_define_module_under(parent, c_name(CONTROLLER_NAME));
    if controller == 0 {
        return;
    }
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_CONNECTED_Q_NAME),
        Some(controller_bool_false),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_NAME_NAME),
        Some(controller_empty_string),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_POWER_NAME),
        Some(controller_power_level),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_AXES_LEFT_NAME),
        Some(controller_zero_axes),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_AXES_RIGHT_NAME),
        Some(controller_zero_axes),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_AXES_TRIGGER_NAME),
        Some(controller_zero_axes),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_RAW_BUTTONS_NAME),
        Some(controller_empty_array),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_RAW_AXES_NAME),
        Some(controller_empty_array),
        0,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_PRESS_EX_NAME),
        Some(controller_bool_false),
        1,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_TRIGGER_EX_NAME),
        Some(controller_bool_false),
        1,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_REPEAT_EX_NAME),
        Some(controller_bool_false),
        1,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_RELEASE_EX_NAME),
        Some(controller_bool_false),
        1,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_REPEATCOUNT_NAME),
        Some(controller_zero),
        1,
    );
    rb_define_module_function(
        controller,
        c_name(CONTROLLER_TIME_EX_NAME),
        Some(controller_float_zero),
        1,
    );
}

unsafe extern "C" fn controller_bool_false(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    rb_sys::Qfalse as VALUE
}

unsafe extern "C" fn controller_zero_axes(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let array = rb_sys::rb_ary_new();
    rb_sys::rb_ary_push(array, rb_float_new(0.0));
    rb_sys::rb_ary_push(array, rb_float_new(0.0));
    array
}

unsafe extern "C" fn controller_empty_array(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    rb_sys::rb_ary_new()
}

unsafe extern "C" fn controller_empty_string(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let empty = CString::new("").expect("empty cstr");
    rb_utf8_str_new(empty.as_ptr(), 0)
}

unsafe extern "C" fn controller_power_level(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    let sym = CString::new("UNKNOWN").expect("symbol");
    rb_id2sym(rb_intern(sym.as_ptr()))
}

unsafe extern "C" fn controller_zero(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    int_to_value(0)
}

unsafe extern "C" fn controller_float_zero(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    rb_float_new(0.0)
}

unsafe extern "C" fn input_delta(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    rb_float_new(current_delta())
}

unsafe extern "C" fn input_update(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    if let Ok(mut store) = STORE.lock() {
        store.advance_frame();
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn input_press_qmark(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(match extract_button(argc, argv) {
        Some(mask) => STORE
            .lock()
            .map(|store| store.is_pressed(mask))
            .unwrap_or(false),
        None => false,
    })
}

unsafe extern "C" fn input_trigger_qmark(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(match extract_button(argc, argv) {
        Some(mask) => STORE
            .lock()
            .map(|store| store.is_triggered(mask))
            .unwrap_or(false),
        None => false,
    })
}

unsafe extern "C" fn input_repeat_qmark(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(match extract_button(argc, argv) {
        Some(mask) => STORE
            .lock()
            .map(|store| store.is_repeated(mask))
            .unwrap_or(false),
        None => false,
    })
}

unsafe extern "C" fn input_release_qmark(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(match extract_button(argc, argv) {
        Some(mask) => STORE
            .lock()
            .map(|store| store.is_released(mask))
            .unwrap_or(false),
        None => false,
    })
}

unsafe extern "C" fn input_count(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let value = match extract_button(argc, argv) {
        Some(mask) => STORE.lock().map(|store| store.count(mask)).unwrap_or(0),
        None => 0,
    };
    int_to_value(value as i64)
}

unsafe extern "C" fn input_time_qmark(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    let seconds = match extract_button(argc, argv) {
        Some(mask) => STORE.lock().map(|store| store.time(mask)).unwrap_or(0.0),
        None => 0.0,
    };
    rb_float_new(seconds)
}

unsafe extern "C" fn input_dir4(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let value = STORE
        .lock()
        .map(|store| store.dir4() as i64)
        .unwrap_or_default();
    int_to_value(value)
}

unsafe extern "C" fn input_dir8(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let value = STORE
        .lock()
        .map(|store| store.dir8() as i64)
        .unwrap_or_default();
    int_to_value(value)
}

unsafe extern "C" fn input_mouse_x(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let x = MOUSE_STATE
        .read()
        .ok()
        .and_then(|state| state.position.map(|pos| pos.0 as i64))
        .unwrap_or(0);
    int_to_value(x)
}

unsafe extern "C" fn input_mouse_y(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let y = MOUSE_STATE
        .read()
        .ok()
        .and_then(|state| state.position.map(|pos| pos.1 as i64))
        .unwrap_or(0);
    int_to_value(y)
}

unsafe extern "C" fn input_scroll_v(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let value = MOUSE_STATE
        .read()
        .map(|state| state.scroll_v as i64)
        .unwrap_or(0);
    int_to_value(value)
}

unsafe extern "C" fn input_mouse_in_window_q(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    bool_to_value(
        MOUSE_STATE
            .read()
            .map(|state| state.in_window)
            .unwrap_or(false),
    )
}

unsafe extern "C" fn input_raw_key_states(
    _argc: c_int,
    _argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    // Raw key states are not tracked yet; return an empty array to remain API-compatible.
    rb_sys::rb_ary_new()
}

unsafe extern "C" fn input_text_input(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    bool_to_value(
        TEXT_STATE
            .read()
            .map(|state| state.enabled)
            .unwrap_or(false),
    )
}

unsafe extern "C" fn input_set_text_input(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qfalse as VALUE;
    }
    let enabled = value_to_bool(*argv);
    if let Ok(mut state) = TEXT_STATE.write() {
        state.enabled = enabled;
        if !enabled {
            state.buffer.clear();
        }
    }
    bool_to_value(enabled)
}

unsafe extern "C" fn input_gets(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    let text = TEXT_STATE
        .write()
        .map(|mut state| std::mem::take(&mut state.buffer))
        .unwrap_or_default();
    let len = text.len() as i64;
    rb_utf8_str_new(text.as_ptr() as *const c_char, len)
}

unsafe extern "C" fn input_clipboard(_argc: c_int, _argv: *const VALUE, _self: VALUE) -> VALUE {
    match clipboard_get() {
        Some(text) => {
            let len = text.len() as i64;
            rb_utf8_str_new(text.as_ptr() as *const c_char, len)
        }
        None => {
            let empty = CString::new("").expect("empty");
            rb_utf8_str_new(empty.as_ptr(), 0)
        }
    }
}

unsafe extern "C" fn input_set_clipboard(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut value = *argv;
    let ptr = rb_string_value_cstr(&mut value);
    if ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let text = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
    let _ = clipboard_set(&text);
    value
}

unsafe fn extract_button(argc: c_int, argv: *const VALUE) -> Option<ButtonMask> {
    if argc != 1 || argv.is_null() {
        return None;
    }
    let button_id = rb_num2int(*argv) as i32;
    BUTTON_TABLE
        .iter()
        .find(|entry| entry.rgss_id == button_id)
        .map(|entry| entry.mask)
}

#[derive(Default)]
struct InputStore {
    current: ButtonMask,
    previous: ButtonMask,
    pending: ButtonMask,
    hold_frames: [u16; BUTTON_COUNT],
    hold_time: [f64; BUTTON_COUNT],
}

impl InputStore {
    fn ingest(&mut self, mask: ButtonMask) {
        self.pending = mask;
    }

    fn advance_frame(&mut self) {
        self.previous = self.current;
        self.current = self.pending;
        let delta = current_delta();
        for (idx, entry) in BUTTON_TABLE.iter().enumerate() {
            if self.current & entry.mask != 0 {
                self.hold_frames[idx] = self.hold_frames[idx].saturating_add(1);
                self.hold_time[idx] += delta;
            } else {
                self.hold_frames[idx] = 0;
                self.hold_time[idx] = 0.0;
            }
        }
    }

    fn is_pressed(&self, mask: ButtonMask) -> bool {
        self.current & mask != 0
    }

    fn is_triggered(&self, mask: ButtonMask) -> bool {
        self.current & mask != 0 && self.previous & mask == 0
    }

    fn is_released(&self, mask: ButtonMask) -> bool {
        self.current & mask == 0 && self.previous & mask != 0
    }

    fn is_repeated(&self, mask: ButtonMask) -> bool {
        if self.is_triggered(mask) {
            return true;
        }
        if let Some(idx) = BUTTON_TABLE.iter().position(|entry| entry.mask == mask) {
            let frames = self.hold_frames[idx];
            frames >= REPEAT_DELAY && (frames - REPEAT_DELAY) % REPEAT_INTERVAL == 0
        } else {
            false
        }
    }

    fn count(&self, mask: ButtonMask) -> u32 {
        if let Some(idx) = BUTTON_TABLE.iter().position(|entry| entry.mask == mask) {
            self.hold_frames[idx] as u32
        } else {
            0
        }
    }

    fn time(&self, mask: ButtonMask) -> f64 {
        if let Some(idx) = BUTTON_TABLE.iter().position(|entry| entry.mask == mask) {
            self.hold_time[idx]
        } else {
            0.0
        }
    }

    fn dir4(&self) -> i32 {
        if self.is_pressed(BUTTON_DOWN) {
            2
        } else if self.is_pressed(BUTTON_UP) {
            8
        } else if self.is_pressed(BUTTON_LEFT) {
            4
        } else if self.is_pressed(BUTTON_RIGHT) {
            6
        } else {
            0
        }
    }

    fn dir8(&self) -> i32 {
        let vertical = if self.is_pressed(BUTTON_DOWN) {
            2
        } else if self.is_pressed(BUTTON_UP) {
            8
        } else {
            0
        };
        let horizontal = if self.is_pressed(BUTTON_LEFT) {
            4
        } else if self.is_pressed(BUTTON_RIGHT) {
            6
        } else {
            0
        };
        match (vertical, horizontal) {
            (2, 4) => 1,
            (2, 6) => 3,
            (8, 4) => 7,
            (8, 6) => 9,
            (v, 0) => v,
            (0, h) => h,
            _ => 0,
        }
    }
}

#[derive(Default)]
struct MouseState {
    position: Option<(f32, f32)>,
    in_window: bool,
    scroll_v: f32,
}

#[derive(Default)]
struct TextState {
    enabled: bool,
    buffer: String,
}

fn clipboard_get() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

fn clipboard_set(text: &str) -> Result<(), arboard::Error> {
    Clipboard::new()?.set_text(text)
}

fn bool_to_value(value: bool) -> VALUE {
    if value {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

fn int_to_value(value: i64) -> VALUE {
    unsafe {
        if value >= special_consts::FIXNUM_MIN as i64 && value <= special_consts::FIXNUM_MAX as i64
        {
            ((value << rb_sys::ruby_special_consts::RUBY_SPECIAL_SHIFT as i64)
                | rb_sys::ruby_special_consts::RUBY_FIXNUM_FLAG as i64) as VALUE
        } else {
            rb_int2big(value as isize)
        }
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
