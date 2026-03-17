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
