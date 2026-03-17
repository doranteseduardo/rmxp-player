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

#[derive(Clone)]
pub struct AutotileTexture {
    pub image: Arc<RgbaImage>,
    pub small: bool,
    frame_width: u32,
    frame_height: u32,
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
        }
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

    pub fn render(&mut self, frame_index: u64, scene: Option<&TileScene>) -> Result<()> {
        let frame = self.pixels.frame_mut();
        match scene {
            Some(scene) => draw_tile_scene(scene, self.logical_size, frame),
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

fn draw_tile_scene(scene: &TileScene, size: (u32, u32), frame: &mut [u8]) {
    let width = size.0 as usize;
    let height = size.1 as usize;
    let map_width_px = scene.map_width * scene.tile_size;
    let map_height_px = scene.map_height * scene.tile_size;
    if map_width_px == 0 || map_height_px == 0 || scene.layers.is_empty() {
        draw_gradient(size, frame, 0);
        return;
    }
    for y in 0..height {
        let map_y = y * map_height_px / height;
        for x in 0..width {
            let map_x = x * map_width_px / width;
            let tile_x = map_x / scene.tile_size;
            let tile_y = map_y / scene.tile_size;
            let local_x = map_x % scene.tile_size;
            let local_y = map_y % scene.tile_size;
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
                let sample = sample_tile_pixel(scene, tile_id, local_x, local_y);
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
            let offset = (y * width + x) * 4;
            frame[offset..offset + 4].copy_from_slice(&final_color);
        }
    }
}

fn sample_tile_pixel(scene: &TileScene, tile_id: i16, local_x: usize, local_y: usize) -> [u8; 4] {
    if tile_id < 48 {
        return [0, 0, 0, 0];
    }
    if tile_id < 384 {
        return sample_autotile_pixel(scene, tile_id as usize, local_x, local_y);
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
        let out_c =
            (src_c * src_a + dst_c * dst_a * (1.0 - src_a)) / out_a;
        dst[i] = (out_c * 255.0).clamp(0.0, 255.0) as u8;
    }
    dst[3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
}

fn sample_autotile_pixel(scene: &TileScene, tile_id: usize, local_x: usize, local_y: usize) -> [u8; 4] {
    let at_index = tile_id / 48 - 1;
    if at_index >= scene.autotiles.len() {
        return FALLBACK_PIXEL;
    }
    let Some(texture) = scene.autotiles[at_index].as_ref() else {
        return FALLBACK_PIXEL;
    };
    if texture.small {
        return sample_small_autotile(texture, local_x, local_y);
    }
    let pattern = tile_id % 48;
    if pattern >= AUTOTILE_RECTS.len() {
        return FALLBACK_PIXEL;
    }
    let quad = (local_y / 16) * 2 + (local_x / 16);
    let quad = quad.min(3);
    let (rect_x, rect_y) = AUTOTILE_RECTS[pattern][quad];
    let sample_x = rect_x as usize + (local_x % 16);
    let sample_y = rect_y as usize + (local_y % 16);
    if sample_x >= texture.frame_width as usize || sample_y >= texture.frame_height as usize {
        return FALLBACK_PIXEL;
    }
    texture
        .image
        .get_pixel(sample_x as u32, sample_y as u32)
        .0
}

fn sample_small_autotile(texture: &AutotileTexture, local_x: usize, local_y: usize) -> [u8; 4] {
    let (width, height) = texture.image.dimensions();
    if width == 0 || height == 0 {
        return FALLBACK_PIXEL;
    }
    let x = (local_x % texture.frame_width as usize).min(width as usize - 1);
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
