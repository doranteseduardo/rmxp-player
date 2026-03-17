use crate::input::InputState;
use render::{Camera, PlayerMarker, RenderFrame, TileScene};
use std::time::Duration;

const PLAYER_SPEED_TILES_PER_SEC: f32 = 4.0;

pub struct GameState {
    scene: Option<TileScene>,
    player: GamePlayer,
    player_color: [u8; 4],
    viewport: (u32, u32),
}

impl GameState {
    pub fn new(scene: Option<TileScene>, start: (f32, f32), viewport: (u32, u32)) -> Self {
        Self {
            scene,
            player: GamePlayer::new(start),
            player_color: [0, 255, 255, 180],
            viewport,
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
        let camera = self.camera(scene);
        Some(RenderFrame {
            scene,
            camera,
            player_marker: Some(PlayerMarker {
                tile_pos: self.player.position,
                color: self.player_color,
            }),
        })
    }

    fn camera(&self, scene: &TileScene) -> Camera {
        let tile_px = scene.tile_size as f32;
        let map_px_w = scene.map_width as f32 * tile_px;
        let map_px_h = scene.map_height as f32 * tile_px;
        let viewport_w = self.viewport.0 as f32;
        let viewport_h = self.viewport.1 as f32;
        let player_center_x = (self.player.position.0 + 0.5) * tile_px;
        let player_center_y = (self.player.position.1 + 0.5) * tile_px;
        let max_x = (map_px_w - viewport_w).max(0.0);
        let max_y = (map_px_h - viewport_h).max(0.0);
        let origin_x = player_center_x - viewport_w / 2.0;
        let origin_y = player_center_y - viewport_h / 2.0;
        Camera {
            origin: (origin_x.clamp(0.0, max_x), origin_y.clamp(0.0, max_y)),
        }
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
