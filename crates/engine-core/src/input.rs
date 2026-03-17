use rgss_bindings::{
    InputSnapshot, BUTTON_A, BUTTON_B, BUTTON_C, BUTTON_DOWN, BUTTON_LEFT, BUTTON_RIGHT, BUTTON_UP,
};
use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

#[derive(Debug, Clone, Copy)]
pub struct InputState {
    pub dir_x: f32,
    pub dir_y: f32,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    button_a: bool,
    button_b: bool,
    button_c: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            dir_x: 0.0,
            dir_y: 0.0,
            left: false,
            right: false,
            up: false,
            down: false,
            button_a: false,
            button_b: false,
            button_c: false,
        }
    }
}

impl InputState {
    pub fn update_from_helper(&mut self, helper: &WinitInputHelper) {
        self.left = helper.key_held(KeyCode::ArrowLeft) || helper.key_held(KeyCode::KeyA);
        self.right = helper.key_held(KeyCode::ArrowRight) || helper.key_held(KeyCode::KeyD);
        self.up = helper.key_held(KeyCode::ArrowUp) || helper.key_held(KeyCode::KeyW);
        self.down = helper.key_held(KeyCode::ArrowDown) || helper.key_held(KeyCode::KeyS);
        self.button_a = helper.key_held(KeyCode::ShiftLeft) || helper.key_held(KeyCode::ShiftRight);
        self.button_b = helper.key_held(KeyCode::KeyX)
            || helper.key_held(KeyCode::Escape)
            || helper.key_held(KeyCode::Backspace);
        self.button_c = helper.key_held(KeyCode::KeyZ)
            || helper.key_held(KeyCode::Enter)
            || helper.key_held(KeyCode::Space);

        self.dir_x = axis(self.right) - axis(self.left);
        self.dir_y = axis(self.down) - axis(self.up);
    }

    pub fn snapshot(&self) -> InputSnapshot {
        InputSnapshot::default()
            .with_button(BUTTON_LEFT, self.left)
            .with_button(BUTTON_RIGHT, self.right)
            .with_button(BUTTON_UP, self.up)
            .with_button(BUTTON_DOWN, self.down)
            .with_button(BUTTON_A, self.button_a)
            .with_button(BUTTON_B, self.button_b)
            .with_button(BUTTON_C, self.button_c)
    }
}

fn axis(pressed: bool) -> f32 {
    if pressed {
        1.0
    } else {
        0.0
    }
}
