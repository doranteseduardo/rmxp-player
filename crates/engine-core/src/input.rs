use rgss_bindings::{
    InputSnapshot, TextEvent, BUTTON_A, BUTTON_ALT, BUTTON_B, BUTTON_C, BUTTON_CTRL, BUTTON_DOWN,
    BUTTON_F5, BUTTON_F6, BUTTON_F7, BUTTON_F8, BUTTON_F9, BUTTON_L, BUTTON_LEFT,
    BUTTON_MOUSE_LEFT, BUTTON_MOUSE_MIDDLE, BUTTON_MOUSE_RIGHT, BUTTON_MOUSE_X1, BUTTON_MOUSE_X2,
    BUTTON_R, BUTTON_RIGHT, BUTTON_SHIFT, BUTTON_UP, BUTTON_X, BUTTON_Y, BUTTON_Z,
};
use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

#[derive(Debug, Clone)]
pub struct InputState {
    pub dir_x: f32,
    pub dir_y: f32,
    mask: u32,
    mouse_pos: Option<(f32, f32)>,
    mouse_in_window: bool,
    scroll_y: f32,
    text_events: Vec<TextEvent>,
    /// 256-element array indexed by SDL/USB-HID scancode.
    raw_key_states: [bool; 256],
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            dir_x: 0.0,
            dir_y: 0.0,
            mask: 0,
            mouse_pos: None,
            mouse_in_window: false,
            scroll_y: 0.0,
            text_events: Vec::new(),
            raw_key_states: [false; 256],
        }
    }
}

impl InputState {
    pub fn update_from_helper(&mut self, helper: &WinitInputHelper) {
        let mut mask = 0;
        let left = helper.key_held(KeyCode::ArrowLeft) || helper.key_held(KeyCode::KeyA);
        let right = helper.key_held(KeyCode::ArrowRight) || helper.key_held(KeyCode::KeyD);
        let up = helper.key_held(KeyCode::ArrowUp) || helper.key_held(KeyCode::KeyW);
        let down = helper.key_held(KeyCode::ArrowDown) || helper.key_held(KeyCode::KeyS);
        if left {
            mask |= BUTTON_LEFT;
        }
        if right {
            mask |= BUTTON_RIGHT;
        }
        if up {
            mask |= BUTTON_UP;
        }
        if down {
            mask |= BUTTON_DOWN;
        }

        if helper.held_shift() || helper.key_held(KeyCode::KeyZ) {
            mask |= BUTTON_A;
        }
        if helper.key_held(KeyCode::KeyX)
            || helper.key_held(KeyCode::Escape)
            || helper.key_held(KeyCode::Backspace)
        {
            mask |= BUTTON_B;
        }
        if helper.key_held(KeyCode::Enter)
            || helper.key_held(KeyCode::Space)
            || helper.key_held(KeyCode::KeyC)
            || helper.key_held(KeyCode::KeyZ)
        {
            mask |= BUTTON_C;
        }
        if helper.key_held(KeyCode::KeyA) {
            mask |= BUTTON_X;
        }
        if helper.key_held(KeyCode::KeyS) {
            mask |= BUTTON_Y;
        }
        if helper.key_held(KeyCode::KeyD) {
            mask |= BUTTON_Z;
        }
        if helper.key_held(KeyCode::KeyQ) {
            mask |= BUTTON_L;
        }
        if helper.key_held(KeyCode::KeyW) {
            mask |= BUTTON_R;
        }
        if helper.held_shift() {
            mask |= BUTTON_SHIFT;
        }
        if helper.held_control() {
            mask |= BUTTON_CTRL;
        }
        if helper.held_alt() {
            mask |= BUTTON_ALT;
        }
        if helper.key_held(KeyCode::F5) {
            mask |= BUTTON_F5;
        }
        if helper.key_held(KeyCode::F6) {
            mask |= BUTTON_F6;
        }
        if helper.key_held(KeyCode::F7) {
            mask |= BUTTON_F7;
        }
        if helper.key_held(KeyCode::F8) {
            mask |= BUTTON_F8;
        }
        if helper.key_held(KeyCode::F9) {
            mask |= BUTTON_F9;
        }
        if helper.mouse_held(0) {
            mask |= BUTTON_MOUSE_LEFT;
        }
        if helper.mouse_held(2) {
            mask |= BUTTON_MOUSE_MIDDLE;
        }
        if helper.mouse_held(1) {
            mask |= BUTTON_MOUSE_RIGHT;
        }
        if helper.mouse_held(3) {
            mask |= BUTTON_MOUSE_X1;
        }
        if helper.mouse_held(4) {
            mask |= BUTTON_MOUSE_X2;
        }

        self.mask = mask;
        self.dir_x = axis(right) - axis(left);
        self.dir_y = axis(down) - axis(up);

        self.mouse_pos = helper.cursor();
        self.mouse_in_window = self.mouse_pos.is_some();
        self.scroll_y = helper.scroll_diff().1;

        // Build SDL-scancode-indexed key state array for Win32API shim.
        let mut raw = [false; 256];
        const ALL_KEYS: &[KeyCode] = &[
            KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE,
            KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyI, KeyCode::KeyJ,
            KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO,
            KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT,
            KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY,
            KeyCode::KeyZ,
            KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3,
            KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7,
            KeyCode::Digit8, KeyCode::Digit9,
            KeyCode::Enter, KeyCode::Escape, KeyCode::Backspace, KeyCode::Tab, KeyCode::Space,
            KeyCode::Minus, KeyCode::Equal, KeyCode::BracketLeft, KeyCode::BracketRight,
            KeyCode::Backslash, KeyCode::Semicolon, KeyCode::Quote, KeyCode::Backquote,
            KeyCode::Comma, KeyCode::Period, KeyCode::Slash, KeyCode::CapsLock,
            KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4, KeyCode::F5, KeyCode::F6,
            KeyCode::F7, KeyCode::F8, KeyCode::F9, KeyCode::F10, KeyCode::F11, KeyCode::F12,
            KeyCode::PrintScreen, KeyCode::ScrollLock, KeyCode::Pause, KeyCode::Insert,
            KeyCode::Home, KeyCode::PageUp, KeyCode::Delete, KeyCode::End, KeyCode::PageDown,
            KeyCode::ArrowRight, KeyCode::ArrowLeft, KeyCode::ArrowDown, KeyCode::ArrowUp,
            KeyCode::NumLock, KeyCode::NumpadDivide, KeyCode::NumpadMultiply,
            KeyCode::NumpadSubtract, KeyCode::NumpadAdd, KeyCode::NumpadEnter,
            KeyCode::Numpad0, KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3,
            KeyCode::Numpad4, KeyCode::Numpad5, KeyCode::Numpad6, KeyCode::Numpad7,
            KeyCode::Numpad8, KeyCode::Numpad9, KeyCode::NumpadDecimal,
            KeyCode::ControlLeft, KeyCode::ShiftLeft, KeyCode::AltLeft, KeyCode::SuperLeft,
            KeyCode::ControlRight, KeyCode::ShiftRight, KeyCode::AltRight, KeyCode::SuperRight,
        ];
        for &key in ALL_KEYS {
            if helper.key_held(key) {
                if let Some(sc) = keycode_to_sdl_scan(key) {
                    raw[sc as usize] = true;
                }
            }
        }
        self.raw_key_states = raw;
    }

    pub fn push_text_char(&mut self, ch: char) {
        if !ch.is_control() || ch == '\n' {
            self.text_events.push(TextEvent::Insert(ch));
        }
    }

    pub fn push_backspace(&mut self) {
        self.text_events.push(TextEvent::Backspace);
    }

    pub fn snapshot(&mut self) -> InputSnapshot {
        let mut snap = InputSnapshot::default();
        snap.set_mask(self.mask);
        snap.set_mouse(self.mouse_pos, self.mouse_in_window);
        snap.set_scroll(self.scroll_y);
        snap.set_raw_key_states(self.raw_key_states);
        for event in self.text_events.drain(..) {
            snap.push_text_event(event);
        }
        self.scroll_y = 0.0;
        snap
    }
}

fn axis(pressed: bool) -> f32 {
    if pressed {
        1.0
    } else {
        0.0
    }
}

/// Maps a winit `KeyCode` to an SDL2/USB-HID scancode (0x00–0xFF).
/// Returns `None` for keys not in the table.
fn keycode_to_sdl_scan(key: KeyCode) -> Option<u8> {
    // SDL2 scancodes match USB HID usage page 0x07 values.
    let sc: u8 = match key {
        KeyCode::KeyA => 0x04, KeyCode::KeyB => 0x05, KeyCode::KeyC => 0x06,
        KeyCode::KeyD => 0x07, KeyCode::KeyE => 0x08, KeyCode::KeyF => 0x09,
        KeyCode::KeyG => 0x0A, KeyCode::KeyH => 0x0B, KeyCode::KeyI => 0x0C,
        KeyCode::KeyJ => 0x0D, KeyCode::KeyK => 0x0E, KeyCode::KeyL => 0x0F,
        KeyCode::KeyM => 0x10, KeyCode::KeyN => 0x11, KeyCode::KeyO => 0x12,
        KeyCode::KeyP => 0x13, KeyCode::KeyQ => 0x14, KeyCode::KeyR => 0x15,
        KeyCode::KeyS => 0x16, KeyCode::KeyT => 0x17, KeyCode::KeyU => 0x18,
        KeyCode::KeyV => 0x19, KeyCode::KeyW => 0x1A, KeyCode::KeyX => 0x1B,
        KeyCode::KeyY => 0x1C, KeyCode::KeyZ => 0x1D,
        KeyCode::Digit1 => 0x1E, KeyCode::Digit2 => 0x1F, KeyCode::Digit3 => 0x20,
        KeyCode::Digit4 => 0x21, KeyCode::Digit5 => 0x22, KeyCode::Digit6 => 0x23,
        KeyCode::Digit7 => 0x24, KeyCode::Digit8 => 0x25, KeyCode::Digit9 => 0x26,
        KeyCode::Digit0 => 0x27,
        KeyCode::Enter => 0x28, KeyCode::Escape => 0x29, KeyCode::Backspace => 0x2A,
        KeyCode::Tab => 0x2B, KeyCode::Space => 0x2C,
        KeyCode::Minus => 0x2D, KeyCode::Equal => 0x2E,
        KeyCode::BracketLeft => 0x2F, KeyCode::BracketRight => 0x30,
        KeyCode::Backslash => 0x31, KeyCode::Semicolon => 0x33,
        KeyCode::Quote => 0x34, KeyCode::Backquote => 0x35,
        KeyCode::Comma => 0x36, KeyCode::Period => 0x37, KeyCode::Slash => 0x38,
        KeyCode::CapsLock => 0x39,
        KeyCode::F1 => 0x3A, KeyCode::F2 => 0x3B, KeyCode::F3 => 0x3C,
        KeyCode::F4 => 0x3D, KeyCode::F5 => 0x3E, KeyCode::F6 => 0x3F,
        KeyCode::F7 => 0x40, KeyCode::F8 => 0x41, KeyCode::F9 => 0x42,
        KeyCode::F10 => 0x43, KeyCode::F11 => 0x44, KeyCode::F12 => 0x45,
        KeyCode::PrintScreen => 0x46, KeyCode::ScrollLock => 0x47,
        KeyCode::Pause => 0x48, KeyCode::Insert => 0x49, KeyCode::Home => 0x4A,
        KeyCode::PageUp => 0x4B, KeyCode::Delete => 0x4C, KeyCode::End => 0x4D,
        KeyCode::PageDown => 0x4E,
        KeyCode::ArrowRight => 0x4F, KeyCode::ArrowLeft => 0x50,
        KeyCode::ArrowDown => 0x51, KeyCode::ArrowUp => 0x52,
        KeyCode::NumLock => 0x53,
        KeyCode::NumpadDivide => 0x54, KeyCode::NumpadMultiply => 0x55,
        KeyCode::NumpadSubtract => 0x56, KeyCode::NumpadAdd => 0x57,
        KeyCode::NumpadEnter => 0x58,
        KeyCode::Numpad1 => 0x59, KeyCode::Numpad2 => 0x5A, KeyCode::Numpad3 => 0x5B,
        KeyCode::Numpad4 => 0x5C, KeyCode::Numpad5 => 0x5D, KeyCode::Numpad6 => 0x5E,
        KeyCode::Numpad7 => 0x5F, KeyCode::Numpad8 => 0x60, KeyCode::Numpad9 => 0x61,
        KeyCode::Numpad0 => 0x62, KeyCode::NumpadDecimal => 0x63,
        KeyCode::ControlLeft => 0xE0, KeyCode::ShiftLeft => 0xE1,
        KeyCode::AltLeft => 0xE2, KeyCode::SuperLeft => 0xE3,
        KeyCode::ControlRight => 0xE4, KeyCode::ShiftRight => 0xE5,
        KeyCode::AltRight => 0xE6, KeyCode::SuperRight => 0xE7,
        _ => return None,
    };
    Some(sc)
}
