use crate::input::InputState;
use render::{PlayerMarker, RenderFrame, TileScene};
use std::time::Duration;

const PLAYER_SPEED_TILES_PER_SEC: f32 = 4.0;

pub struct GameState {
    scene: Option<TileScene>,
    player: GamePlayer,
    player_color: [u8; 4],
}

impl GameState {
    pub fn new(scene: Option<TileScene>, start: (f32, f32)) -> Self {
        Self {
            scene,
            player: GamePlayer::new(start),
            player_color: [0, 255, 255, 180],
        }
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        if let Some(scene) = self.scene.as_ref() {
            self.player
                .update(dt, input, scene.map_width, scene.map_height);
        }
    }

    pub fn render_frame(&self) -> Option<RenderFrame<'_>> {
        let scene = self.scene.as_ref()?;
        Some(RenderFrame {
            scene,
            player_marker: Some(PlayerMarker {
                tile_pos: self.player.position,
                color: self.player_color,
            }),
        })
    }
}

struct GamePlayer {
    position: (f32, f32),
    speed: f32,
}

impl GamePlayer {
    fn new(start: (f32, f32)) -> Self {
        Self {
            position: start,
            speed: PLAYER_SPEED_TILES_PER_SEC,
        }
    }

    fn update(&mut self, dt: Duration, input: &InputState, width: usize, height: usize) {
        let mut dir_x = input.dir_x;
        let mut dir_y = input.dir_y;
        if dir_x != 0.0 || dir_y != 0.0 {
            let len = (dir_x * dir_x + dir_y * dir_y)
                .sqrt()
                .max(std::f32::EPSILON);
            dir_x /= len;
            dir_y /= len;
        }
        let delta = self.speed * dt.as_secs_f32();
        let mut x = self.position.0 + dir_x * delta;
        let mut y = self.position.1 + dir_y * delta;
        let max_x = width.max(1) as f32 - 0.01;
        let max_y = height.max(1) as f32 - 0.01;
        x = x.clamp(0.0, max_x);
        y = y.clamp(0.0, max_y);
        self.position = (x, y);
    }
}
