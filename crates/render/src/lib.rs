use anyhow::Result;
use image::RgbaImage;
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{dpi::PhysicalSize, window::Window};

const FALLBACK_PIXEL: [u8; 4] = [255, 0, 255, 0xFF];

/// Basic renderer responsible for presenting frames using `pixels`.
pub struct Renderer<'a> {
    pixels: Pixels<'a>,
    logical_size: (u32, u32),
}

pub struct RenderFrame<'a> {
    pub scene: &'a TileScene,
    pub camera: Camera,
    pub player_marker: Option<PlayerMarker>,
    pub sprites: &'a [SpriteInstance],
}

#[derive(Clone, Copy)]
pub struct Camera {
    pub origin: (f32, f32),
}

#[derive(Clone, Copy)]
pub struct PlayerMarker {
    pub tile_pos: (f32, f32),
    pub color: [u8; 4],
}

#[derive(Clone)]
pub struct SpriteInstance {
    pub texture: Arc<RgbaImage>,
    pub screen_pos: (i32, i32),
    pub src_rect: (u32, u32, u32, u32),
    pub opacity: u8,
    pub z: i32,
}

#[derive(Clone)]
pub struct AutotileTexture {
    pub image: Arc<RgbaImage>,
    pub small: bool,
    frame_width: u32,
    frame_height: u32,
    frames: u32,
}

impl AutotileTexture {
    pub fn new(image: Arc<RgbaImage>) -> Self {
        let (width, height) = image.dimensions();
        let small = height <= 32;
        let base_width = if small { 32 } else { 96 };
        let base_height = if small { 32 } else { 128 };
        Self {
            image,
            small,
            frame_width: base_width.min(width.max(1)),
            frame_height: base_height.min(height.max(1)),
            frames: (width / base_width).max(1),
        }
    }

    fn current_frame(&self, frame_index: u64) -> u32 {
        if self.frames <= 1 {
            return 0;
        }
        const FRAME_INTERVAL: u64 = 8;
        ((frame_index / FRAME_INTERVAL) % self.frames as u64) as u32
    }
}

pub struct TileScene {
    pub map_width: usize,
    pub map_height: usize,
    pub tile_size: usize,
    pub tileset: Arc<RgbaImage>,
    pub autotiles: Vec<Option<AutotileTexture>>,
    pub layers: Vec<Vec<i16>>,
    pub priorities: Vec<u8>,
}

impl TileScene {
    fn tile_priority(&self, tile_id: i16) -> u8 {
        if tile_id < 0 {
            return 0;
        }
        let index = tile_id as usize;
        self.priorities.get(index).copied().unwrap_or(0)
    }
}

impl<'a> Renderer<'a> {
    pub fn new(window: &'a Window, logical_width: u32, logical_height: u32) -> Result<Self> {
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width, size.height, window);
        let pixels = Pixels::new(logical_width, logical_height, surface)?;
        Ok(Self {
            pixels,
            logical_size: (logical_width, logical_height),
        })
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        let _ = self.pixels.resize_surface(size.width, size.height);
        let _ = self
            .pixels
            .resize_buffer(self.logical_size.0, self.logical_size.1);
    }

    pub fn render(&mut self, frame_index: u64, frame_data: Option<RenderFrame<'_>>) -> Result<()> {
        let frame = self.pixels.frame_mut();
        match frame_data {
            Some(data) => {
                draw_tile_scene(
                    data.scene,
                    data.camera,
                    data.player_marker.as_ref(),
                    self.logical_size,
                    frame,
                    frame_index,
                );
                draw_sprites(self.logical_size, frame, data.sprites);
            }
            None => draw_gradient(self.logical_size, frame, frame_index),
        }
        self.pixels.render()?;
        Ok(())
    }
}

fn draw_gradient(size: (u32, u32), frame: &mut [u8], frame_index: u64) {
    let width = size.0 as usize;
    for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
        let x = (i % width) as u8;
        let y = (i / width) as u8;
        let t = frame_index as u8;
        pixel[0] = x.wrapping_add(t);
        pixel[1] = y.wrapping_add(t);
        pixel[2] = 0x80;
        pixel[3] = 0xFF;
    }
}

fn debug_pixel(x: usize, y: usize, width: usize) -> [u8; 4] {
    let width = width.max(1);
    let xx = (x % width) as u8;
    let yy = (y % width) as u8;
    [xx, yy, 0x80, 0xFF]
}

fn set_pixel(frame: &mut [u8], width: usize, x: usize, y: usize, color: [u8; 4]) {
    let offset = (y * width + x) * 4;
    frame[offset..offset + 4].copy_from_slice(&color);
}

fn fill_background_line(frame: &mut [u8], width: usize, y: usize) {
    for x in 0..width {
        set_pixel(frame, width, x, y, debug_pixel(x, y, width));
    }
}

fn draw_tile_scene(
    scene: &TileScene,
    camera: Camera,
    player_marker: Option<&PlayerMarker>,
    size: (u32, u32),
    frame: &mut [u8],
    frame_index: u64,
) {
    let width = size.0 as usize;
    let height = size.1 as usize;
    let map_width_px = scene.map_width * scene.tile_size;
    let map_height_px = scene.map_height * scene.tile_size;
    if map_width_px == 0 || map_height_px == 0 || scene.layers.is_empty() {
        draw_gradient(size, frame, 0);
        return;
    }
    let cam_x = camera.origin.0.max(0.0);
    let cam_y = camera.origin.1.max(0.0);
    for y in 0..height {
        let world_y = cam_y + y as f32;
        if world_y < 0.0 || world_y >= map_height_px as f32 {
            fill_background_line(frame, width, y);
            continue;
        }
        let tile_y = (world_y as usize) / scene.tile_size;
        if tile_y >= scene.map_height {
            fill_background_line(frame, width, y);
            continue;
        }
        let local_y = (world_y as usize) % scene.tile_size;
        for x in 0..width {
            let world_x = cam_x + x as f32;
            if world_x < 0.0 || world_x >= map_width_px as f32 {
                set_pixel(frame, width, x, y, debug_pixel(x, y, width));
                continue;
            }
            let tile_x = (world_x as usize) / scene.tile_size;
            if tile_x >= scene.map_width {
                set_pixel(frame, width, x, y, debug_pixel(x, y, width));
                continue;
            }
            let local_x = (world_x as usize) % scene.tile_size;
            let mut ground = [0, 0, 0, 0];
            let mut overlay = [0, 0, 0, 0];
            for layer in &scene.layers {
                let idx = tile_y * scene.map_width + tile_x;
                if idx >= layer.len() {
                    continue;
                }
                let tile_id = layer[idx];
                if tile_id < 48 {
                    continue;
                }
                let sample = sample_tile_pixel(scene, tile_id, local_x, local_y, frame_index);
                if sample[3] == 0 {
                    continue;
                }
                let priority = scene.tile_priority(tile_id);
                if priority == 0 {
                    blend_pixel(&mut ground, sample);
                } else {
                    blend_pixel(&mut overlay, sample);
                }
            }
            let mut final_color = ground;
            if overlay[3] > 0 {
                blend_pixel(&mut final_color, overlay);
            }
            if final_color[3] == 0 {
                final_color = debug_pixel(x, y, width);
            }
            set_pixel(frame, width, x, y, final_color);
        }
    }
    if let Some(marker) = player_marker {
        draw_player_marker(scene, camera, marker, size, frame);
    }
}

fn sample_tile_pixel(
    scene: &TileScene,
    tile_id: i16,
    local_x: usize,
    local_y: usize,
    frame_index: u64,
) -> [u8; 4] {
    if tile_id < 48 {
        return [0, 0, 0, 0];
    }
    if tile_id < 384 {
        return sample_autotile_pixel(scene, tile_id as usize, local_x, local_y, frame_index);
    }
    let tile_index = (tile_id - 384) as usize;
    let tile_size = scene.tile_size;
    let tileset = &scene.tileset;
    let tiles_per_row = (tileset.width() as usize / tile_size.max(1)).max(1);
    let src_x = (tile_index % tiles_per_row) * tile_size + local_x;
    let src_y = (tile_index / tiles_per_row) * tile_size + local_y;
    if src_x >= tileset.width() as usize || src_y >= tileset.height() as usize {
        return [0, 0, 0, 0xFF];
    }
    tileset.get_pixel(src_x as u32, src_y as u32).0
}

fn blend_pixel(dst: &mut [u8; 4], src: [u8; 4]) {
    let src_a = src[3] as f32 / 255.0;
    if src_a <= 0.0 {
        return;
    }
    if (dst[3] == 0 && src_a >= 1.0) || (dst[3] == 255 && src_a >= 1.0) {
        *dst = src;
        return;
    }
    let dst_a = dst[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= 0.0 {
        *dst = [0, 0, 0, 0];
        return;
    }
    for i in 0..3 {
        let src_c = src[i] as f32 / 255.0;
        let dst_c = dst[i] as f32 / 255.0;
        let out_c = (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        dst[i] = (out_c * 255.0).clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
}

fn draw_player_marker(
    scene: &TileScene,
    camera: Camera,
    marker: &PlayerMarker,
    size: (u32, u32),
    frame: &mut [u8],
) {
    let tile_px = scene.tile_size as f32;
    let player_px_x = (marker.tile_pos.0 + 0.5) * tile_px;
    let player_px_y = (marker.tile_pos.1 + 0.5) * tile_px;
    let screen_x = player_px_x - camera.origin.0;
    let screen_y = player_px_y - camera.origin.1;
    if screen_x < 0.0 || screen_y < 0.0 || screen_x >= size.0 as f32 || screen_y >= size.1 as f32 {
        return;
    }
    let center_x = screen_x as isize;
    let center_y = screen_y as isize;
    let marker_size = 6isize;
    let width = size.0 as usize;
    for y in (center_y - marker_size)..=(center_y + marker_size) {
        if y < 0 || y >= size.1 as isize {
            continue;
        }
        for x in (center_x - marker_size)..=(center_x + marker_size) {
            if x < 0 || x >= size.0 as isize {
                continue;
            }
            let offset = (y as usize * width + x as usize) * 4;
            let dst = &mut frame[offset..offset + 4];
            let mut color = [dst[0], dst[1], dst[2], dst[3]];
            blend_pixel(&mut color, marker.color);
            dst.copy_from_slice(&color);
        }
    }
}

fn draw_sprites(size: (u32, u32), frame: &mut [u8], sprites: &[SpriteInstance]) {
    if sprites.is_empty() {
        return;
    }
    for sprite in sprites {
        draw_sprite(size, frame, sprite);
    }
}

fn draw_sprite(size: (u32, u32), frame: &mut [u8], sprite: &SpriteInstance) {
    if sprite.opacity == 0 {
        return;
    }
    let width = size.0 as i32;
    let height = size.1 as i32;
    let tex_width = sprite.texture.width();
    let tex_height = sprite.texture.height();
    for sy in 0..sprite.src_rect.3 {
        let dest_y = sprite.screen_pos.1 + sy as i32;
        if dest_y < 0 || dest_y >= height {
            continue;
        }
        let src_y = sprite.src_rect.1 + sy;
        if src_y >= tex_height {
            continue;
        }
        for sx in 0..sprite.src_rect.2 {
            let dest_x = sprite.screen_pos.0 + sx as i32;
            if dest_x < 0 || dest_x >= width {
                continue;
            }
            let src_x = sprite.src_rect.0 + sx;
            if src_x >= tex_width {
                continue;
            }
            let mut color = sprite.texture.get_pixel(src_x, src_y).0;
            if color[3] == 0 {
                continue;
            }
            if sprite.opacity < 255 {
                let alpha = (color[3] as u16 * sprite.opacity as u16).saturating_div(255) as u8;
                color[3] = alpha;
            }
            let idx = (dest_y as usize * size.0 as usize + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_pixel(&mut dst, color);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn sample_autotile_pixel(
    scene: &TileScene,
    tile_id: usize,
    local_x: usize,
    local_y: usize,
    frame_index: u64,
) -> [u8; 4] {
    let at_index = tile_id / 48 - 1;
    if at_index >= scene.autotiles.len() {
        return FALLBACK_PIXEL;
    }
    let Some(texture) = scene.autotiles[at_index].as_ref() else {
        return FALLBACK_PIXEL;
    };
    if texture.small {
        return sample_small_autotile(texture, local_x, local_y, frame_index);
    }
    let pattern = tile_id % 48;
    if pattern >= AUTOTILE_RECTS.len() {
        return FALLBACK_PIXEL;
    }
    let frame = texture.current_frame(frame_index);
    let frame_offset = frame as usize * texture.frame_width as usize;
    let quad = (local_y / 16) * 2 + (local_x / 16);
    let quad = quad.min(3);
    let (rect_x, rect_y) = AUTOTILE_RECTS[pattern][quad];
    let sample_x = frame_offset + rect_x as usize + (local_x % 16);
    let sample_y = rect_y as usize + (local_y % 16);
    if sample_x >= texture.image.width() as usize || sample_y >= texture.image.height() as usize {
        return FALLBACK_PIXEL;
    }
    texture.image.get_pixel(sample_x as u32, sample_y as u32).0
}

fn sample_small_autotile(
    texture: &AutotileTexture,
    local_x: usize,
    local_y: usize,
    frame_index: u64,
) -> [u8; 4] {
    let (width, height) = texture.image.dimensions();
    if width == 0 || height == 0 {
        return FALLBACK_PIXEL;
    }
    let frame = texture.current_frame(frame_index);
    let frame_offset = frame as usize * texture.frame_width as usize;
    let x = (frame_offset + (local_x % texture.frame_width as usize)).min(width as usize - 1);
    let y = (local_y % texture.frame_height as usize).min(height as usize - 1);
    texture.image.get_pixel(x as u32, y as u32).0
}

const AUTOTILE_RECTS: [[(u32, u32); 4]; 48] = [
    [(32, 64), (48, 64), (32, 80), (48, 80)],
    [(64, 0), (48, 64), (32, 80), (48, 80)],
    [(32, 64), (80, 0), (32, 80), (48, 80)],
    [(64, 0), (80, 0), (32, 80), (48, 80)],
    [(32, 64), (48, 64), (32, 80), (80, 16)],
    [(64, 0), (48, 64), (32, 80), (80, 16)],
    [(32, 64), (80, 0), (32, 80), (80, 16)],
    [(64, 0), (80, 0), (32, 80), (80, 16)],
    [(32, 64), (48, 64), (64, 16), (48, 80)],
    [(64, 0), (48, 64), (64, 16), (48, 80)],
    [(32, 64), (80, 0), (64, 16), (48, 80)],
    [(64, 0), (80, 0), (64, 16), (48, 80)],
    [(32, 64), (48, 64), (64, 16), (80, 16)],
    [(64, 0), (48, 64), (64, 16), (80, 16)],
    [(32, 64), (80, 0), (64, 16), (80, 16)],
    [(64, 0), (80, 0), (64, 16), (80, 16)],
    [(0, 64), (16, 64), (0, 80), (16, 80)],
    [(0, 64), (80, 0), (0, 80), (16, 80)],
    [(0, 64), (16, 64), (0, 80), (80, 16)],
    [(0, 64), (80, 0), (0, 80), (80, 16)],
    [(32, 32), (48, 32), (32, 48), (48, 48)],
    [(32, 32), (48, 32), (32, 48), (80, 16)],
    [(32, 32), (48, 32), (64, 16), (48, 48)],
    [(32, 32), (48, 32), (64, 16), (80, 16)],
    [(64, 64), (80, 64), (64, 80), (80, 80)],
    [(64, 64), (80, 64), (64, 16), (80, 80)],
    [(64, 0), (80, 64), (64, 80), (80, 80)],
    [(64, 0), (80, 64), (64, 16), (80, 80)],
    [(32, 96), (48, 96), (32, 112), (48, 112)],
    [(64, 0), (48, 96), (32, 112), (48, 112)],
    [(32, 96), (80, 0), (32, 112), (48, 112)],
    [(64, 0), (80, 0), (32, 112), (48, 112)],
    [(0, 64), (80, 64), (0, 80), (80, 80)],
    [(32, 32), (48, 32), (32, 112), (48, 112)],
    [(0, 32), (16, 32), (0, 48), (16, 48)],
    [(0, 32), (16, 32), (0, 48), (80, 16)],
    [(64, 32), (80, 32), (64, 48), (80, 48)],
    [(64, 32), (80, 32), (64, 16), (80, 48)],
    [(64, 96), (80, 96), (64, 112), (80, 112)],
    [(64, 0), (80, 96), (64, 112), (80, 112)],
    [(0, 96), (16, 96), (0, 112), (16, 112)],
    [(0, 96), (80, 0), (0, 112), (16, 112)],
    [(0, 32), (80, 32), (0, 48), (80, 48)],
    [(0, 32), (16, 32), (0, 112), (16, 112)],
    [(0, 96), (80, 96), (0, 112), (80, 112)],
    [(64, 32), (80, 32), (64, 112), (80, 112)],
    [(0, 32), (80, 32), (0, 112), (80, 112)],
    [(0, 0), (16, 0), (0, 16), (16, 16)],
];
