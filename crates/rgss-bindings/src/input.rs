use anyhow::{anyhow, Result};
use once_cell::sync::{Lazy, OnceCell};
use rb_sys::{
    rb_define_module, rb_int2big, rb_num2int, ruby_special_consts, special_consts, VALUE,
};
use std::{
    os::raw::{c_char, c_int},
    sync::Mutex,
};

const INPUT_NAME: &[u8] = b"Input\0";
const UPDATE_NAME: &[u8] = b"update\0";
const PRESS_Q_NAME: &[u8] = b"press?\0";
const TRIGGER_Q_NAME: &[u8] = b"trigger?\0";
const REPEAT_Q_NAME: &[u8] = b"repeat?\0";
const DIR4_NAME: &[u8] = b"dir4\0";
const DIR8_NAME: &[u8] = b"dir8\0";

const REPEAT_DELAY: u8 = 20;
const REPEAT_INTERVAL: u8 = 5;

#[derive(Clone, Copy)]
struct ButtonConst {
    name: &'static [u8],
    rgss_id: i32,
    mask: u16,
}

const BUTTON_TABLE: [ButtonConst; 7] = [
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
];

static INPUT_MODULE: OnceCell<()> = OnceCell::new();
static STORE: Lazy<Mutex<InputStore>> = Lazy::new(|| Mutex::new(InputStore::default()));

pub const BUTTON_DOWN: u16 = 1 << 0;
pub const BUTTON_LEFT: u16 = 1 << 1;
pub const BUTTON_RIGHT: u16 = 1 << 2;
pub const BUTTON_UP: u16 = 1 << 3;
pub const BUTTON_A: u16 = 1 << 4;
pub const BUTTON_B: u16 = 1 << 5;
pub const BUTTON_C: u16 = 1 << 6;

type RubyFn = unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE;

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<RubyFn>,
        argc: c_int,
    );
    fn rb_define_const(module: VALUE, name: *const c_char, value: VALUE);
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InputSnapshot {
    mask: u16,
}

impl InputSnapshot {
    pub fn with_button(mut self, button: u16, pressed: bool) -> Self {
        self.set_button(button, pressed);
        self
    }

    pub fn set_button(&mut self, button: u16, pressed: bool) {
        if pressed {
            self.mask |= button;
        } else {
            self.mask &= !button;
        }
    }
}

impl InputSnapshot {
    pub(crate) fn mask(&self) -> u16 {
        self.mask
    }
}

pub fn init() -> Result<()> {
    INPUT_MODULE
        .get_or_try_init(|| unsafe { define_input() })
        .map(|_| ())
}

pub fn update_input(snapshot: InputSnapshot) {
    if let Ok(mut store) = STORE.lock() {
        store.ingest(snapshot.mask());
    }
}

unsafe fn define_input() -> Result<()> {
    let module = rb_define_module(c_name(INPUT_NAME));
    if module == 0 {
        return Err(anyhow!("failed to define Input module"));
    }

    rb_define_module_function(module, c_name(UPDATE_NAME), Some(input_update), -1);
    rb_define_module_function(module, c_name(PRESS_Q_NAME), Some(input_press_qmark), 1);
    rb_define_module_function(module, c_name(TRIGGER_Q_NAME), Some(input_trigger_qmark), 1);
    rb_define_module_function(module, c_name(REPEAT_Q_NAME), Some(input_repeat_qmark), 1);
    rb_define_module_function(module, c_name(DIR4_NAME), Some(input_dir4), 0);
    rb_define_module_function(module, c_name(DIR8_NAME), Some(input_dir8), 0);

    for entry in BUTTON_TABLE {
        rb_define_const(
            module,
            c_name(entry.name),
            int_to_value(entry.rgss_id as i64),
        );
    }
    Ok(())
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

unsafe fn extract_button(argc: c_int, argv: *const VALUE) -> Option<u16> {
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
    current: u16,
    previous: u16,
    pending: u16,
    repeat_frames: [u8; BUTTON_TABLE.len()],
}

impl InputStore {
    fn ingest(&mut self, mask: u16) {
        self.pending = mask;
    }

    fn advance_frame(&mut self) {
        self.previous = self.current;
        self.current = self.pending;

        for (idx, entry) in BUTTON_TABLE.iter().enumerate() {
            if self.current & entry.mask != 0 {
                let next = self.repeat_frames[idx].saturating_add(1);
                self.repeat_frames[idx] = next;
            } else {
                self.repeat_frames[idx] = 0;
            }
        }
    }

    fn is_pressed(&self, mask: u16) -> bool {
        self.current & mask != 0
    }

    fn is_triggered(&self, mask: u16) -> bool {
        self.current & mask != 0 && self.previous & mask == 0
    }

    fn is_repeated(&self, mask: u16) -> bool {
        if self.is_triggered(mask) {
            return true;
        }
        if let Some(idx) = BUTTON_TABLE.iter().position(|entry| entry.mask == mask) {
            let frames = self.repeat_frames[idx];
            frames >= REPEAT_DELAY && (frames - REPEAT_DELAY) % REPEAT_INTERVAL == 0
        } else {
            false
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
            ((value << ruby_special_consts::RUBY_SPECIAL_SHIFT as i64)
                | ruby_special_consts::RUBY_FIXNUM_FLAG as i64) as VALUE
        } else {
            rb_int2big(value as isize)
        }
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}
