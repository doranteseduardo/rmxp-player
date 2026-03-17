use crate::input::InputState;
use image::RgbaImage;
use render::{AutotileTexture, Camera, PlayerMarker, RenderFrame, SpriteInstance, TileScene};
use rgss_bindings::{bitmap_snapshot, sprite_snapshot, tilemap_snapshot, TilemapData};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

const PLAYER_SPEED_TILES_PER_SEC: f32 = 4.0;

pub struct GameState {
    scene: Option<TileScene>,
    player: GamePlayer,
    player_color: [u8; 4],
    viewport: (u32, u32),
    sprites: Vec<SpriteInstance>,
    override_origin: Option<(f32, f32)>,
}

impl GameState {
    pub fn new(scene: Option<TileScene>, start: (f32, f32), viewport: (u32, u32)) -> Self {
        Self {
            scene,
            player: GamePlayer::new(start),
            player_color: [0, 255, 255, 180],
            viewport,
            sprites: Vec::new(),
            override_origin: None,
        }
    }

    pub fn update(&mut self, dt: Duration, input: &InputState) {
        if let Some(scene) = self.scene.as_ref() {
            self.player
                .update(dt, input, scene.map_width, scene.map_height);
        }
    }

    pub fn render_frame(&mut self) -> Option<RenderFrame<'_>> {
        let textures = self.collect_textures();
        self.sync_scene_from_tilemap(&textures);
        let scene_ptr = self.scene.as_ref()? as *const TileScene;
        let camera = if let Some(origin) = self.override_origin {
            Camera { origin }
        } else {
            self.camera(unsafe { &*scene_ptr })
        };
        self.rebuild_rgss_sprites(&textures, &camera);
        let scene = unsafe { &*scene_ptr };
        Some(RenderFrame {
            scene,
            camera,
            player_marker: self.player_marker(),
            sprites: &self.sprites,
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

    fn rebuild_rgss_sprites(&mut self, textures: &HashMap<u32, Arc<RgbaImage>>, camera: &Camera) {
        self.sprites.clear();
        for (_id, sprite) in sprite_snapshot() {
            if sprite.disposed || !sprite.visible {
                continue;
            }
            let Some(bitmap_id) = sprite.bitmap_id else {
                continue;
            };
            let Some(texture) = textures.get(&bitmap_id).cloned() else {
                continue;
            };
            let opacity = sprite.opacity.clamp(0, 255) as u8;
            if opacity == 0 {
                continue;
            }
            let src_w = if sprite.src_rect.width > 0 {
                sprite.src_rect.width.max(0) as u32
            } else {
                texture.width()
            };
            let src_h = if sprite.src_rect.height > 0 {
                sprite.src_rect.height.max(0) as u32
            } else {
                texture.height()
            };
            if src_w == 0 || src_h == 0 {
                continue;
            }
            let src_x = sprite.src_rect.x.max(0) as u32;
            let src_y = sprite.src_rect.y.max(0) as u32;
            let world_x = sprite.x - sprite.ox;
            let world_y = sprite.y - sprite.oy;
            let screen_x = world_x - camera.origin.0;
            let screen_y = world_y - camera.origin.1;
            let instance = SpriteInstance {
                texture,
                screen_pos: (screen_x.round() as i32, screen_y.round() as i32),
                src_rect: (src_x, src_y, src_w, src_h),
                opacity,
                z: sprite.z,
            };
            self.sprites.push(instance);
        }
        self.sprites.sort_by_key(|sprite| sprite.z);
    }

    fn collect_textures(&self) -> HashMap<u32, Arc<RgbaImage>> {
        bitmap_snapshot()
            .into_iter()
            .filter(|(_, data)| !data.disposed)
            .map(|(id, data)| (id, data.texture))
            .collect()
    }

    fn sync_scene_from_tilemap(&mut self, textures: &HashMap<u32, Arc<RgbaImage>>) {
        let tilemaps = tilemap_snapshot();
        for (_id, tilemap) in tilemaps {
            if tilemap.disposed || !tilemap.visible {
                continue;
            }
            if let Some(scene) = self.scene_from_tilemap(&tilemap, textures) {
                self.scene = Some(scene);
                self.override_origin = Some((tilemap.ox as f32, tilemap.oy as f32));
                return;
            }
        }
        self.override_origin = None;
    }

    fn scene_from_tilemap(
        &self,
        tilemap: &TilemapData,
        textures: &HashMap<u32, Arc<RgbaImage>>,
    ) -> Option<TileScene> {
        let grid = tilemap.map.as_ref()?;
        let tileset_id = tilemap.tileset_id?;
        let tileset = textures.get(&tileset_id)?.clone();
        let autotiles = tilemap
            .autotile_ids
            .iter()
            .map(|handle| {
                handle
                    .and_then(|id| textures.get(&id).cloned())
                    .map(AutotileTexture::new)
            })
            .collect::<Vec<_>>();
        let priorities = if tilemap.priorities.is_empty() {
            Vec::new()
        } else {
            tilemap
                .priorities
                .iter()
                .map(|value| (*value).clamp(0, 6) as u8)
                .collect()
        };
        Some(TileScene {
            map_width: grid.width,
            map_height: grid.height,
            tile_size: 32,
            tileset,
            autotiles,
            layers: grid.layers.clone(),
            priorities,
        })
    }

    fn player_marker(&self) -> Option<PlayerMarker> {
        if self.override_origin.is_some() {
            None
        } else {
            Some(PlayerMarker {
                tile_pos: self.player.position,
                color: self.player_color,
            })
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
