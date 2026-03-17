use winit::keyboard::KeyCode;
use winit_input_helper::WinitInputHelper;

#[derive(Debug, Clone, Copy, Default)]
pub struct InputState {
    pub dir_x: f32,
    pub dir_y: f32,
    pub confirm: bool,
}

impl InputState {
    pub fn update_from_helper(&mut self, helper: &WinitInputHelper) {
        let left = helper.key_held(KeyCode::ArrowLeft) || helper.key_held(KeyCode::KeyA);
        let right = helper.key_held(KeyCode::ArrowRight) || helper.key_held(KeyCode::KeyD);
        let up = helper.key_held(KeyCode::ArrowUp) || helper.key_held(KeyCode::KeyW);
        let down = helper.key_held(KeyCode::ArrowDown) || helper.key_held(KeyCode::KeyS);
        self.dir_x = axis(right) - axis(left);
        self.dir_y = axis(down) - axis(up);
        self.confirm = helper.key_pressed(KeyCode::Enter) || helper.key_pressed(KeyCode::Space);
    }
}

fn axis(pressed: bool) -> f32 {
    if pressed {
        1.0
    } else {
        0.0
    }
}
