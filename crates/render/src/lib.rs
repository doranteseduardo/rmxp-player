use anyhow::Result;
use image::RgbaImage;
use pixels::{Pixels, SurfaceTexture};
use rgss_bindings::{screen_effects, store_backbuffer, ScreenEffects};
use std::sync::Arc;
use winit::{dpi::PhysicalSize, window::Window};

const FALLBACK_PIXEL: [u8; 4] = [255, 0, 255, 0xFF];
const WINDOW_PADDING: i32 = 16;

#[derive(Clone, Copy)]
struct SkinRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

const WINDOW_BG_SRC: SkinRect = SkinRect {
    x: 0,
    y: 0,
    w: 64,
    h: 64,
};
const WINDOW_CORNER_SRC: [SkinRect; 4] = [
    SkinRect {
        x: 64,
        y: 0,
        w: 16,
        h: 16,
    },
    SkinRect {
        x: 112,
        y: 0,
        w: 16,
        h: 16,
    },
    SkinRect {
        x: 64,
        y: 48,
        w: 16,
        h: 16,
    },
    SkinRect {
        x: 112,
        y: 48,
        w: 16,
        h: 16,
    },
];
const WINDOW_EDGE_SRC: [SkinRect; 4] = [
    SkinRect {
        x: 80,
        y: 0,
        w: 32,
        h: 16,
    }, // top
    SkinRect {
        x: 80,
        y: 48,
        w: 32,
        h: 16,
    }, // bottom
    SkinRect {
        x: 64,
        y: 16,
        w: 16,
        h: 32,
    }, // left
    SkinRect {
        x: 112,
        y: 16,
        w: 16,
        h: 32,
    }, // right
];
const WINDOW_CURSOR_SRC: SkinRect = SkinRect {
    x: 64,
    y: 64,
    w: 32,
    h: 32,
};

/// Basic renderer responsible for presenting frames using `pixels`.
pub struct Renderer<'a> {
    pixels: Pixels<'a>,
    logical_size: (u32, u32),
}

pub struct RenderFrame<'a> {
    pub tilemaps: &'a [TilemapInstance],
    pub planes: &'a [PlaneInstance],
    pub sprites: &'a [SpriteInstance],
    pub windows: &'a [WindowInstance],
    pub fallback: Option<FallbackScene<'a>>,
}

pub struct FallbackScene<'a> {
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

#[derive(Clone, Copy, Debug)]
pub struct ClipRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone)]
pub struct SpriteInstance {
    pub texture: Arc<RgbaImage>,
    pub src_rect: (u32, u32, u32, u32),
    pub origin: (f32, f32),
    pub position: (f32, f32),
    pub opacity: u8,
    pub z: i32,
    pub scale: (f32, f32),
    pub angle: f32,
    pub mirror: bool,
    pub tone: [f32; 4],
    pub color: [f32; 4],
    pub blend_type: u8,
    pub bush_depth: u32,
    pub bush_opacity: u8,
    pub clip: ClipRect,
}

#[derive(Clone)]
pub struct TilemapInstance {
    pub scene: TileScene,
    pub camera: Camera,
    pub clip: ClipRect,
    pub z: i32,
    pub tone: [f32; 4],
    pub color: [f32; 4],
    pub opacity: u8,
    pub blend_type: u8,
}

#[derive(Clone)]
pub struct PlaneInstance {
    pub texture: Arc<RgbaImage>,
    pub clip: ClipRect,
    pub scroll: (f32, f32),
    pub zoom: (f32, f32),
    pub opacity: u8,
    pub blend_type: u8,
    pub tone: [f32; 4],
    pub color: [f32; 4],
    pub z: i32,
}

#[derive(Clone)]
pub struct WindowInstance {
    pub frame_rect: ClipRect,
    pub visible_rect: ClipRect,
    pub clip: ClipRect,
    pub windowskin: Option<Arc<RgbaImage>>,
    pub contents: Option<Arc<RgbaImage>>,
    pub contents_origin: (i32, i32),
    pub opacity: u8,
    pub back_opacity: u8,
    pub contents_opacity: u8,
    pub tone: [f32; 4],
    pub color: [f32; 4],
    pub cursor_rect: Option<ClipRect>,
    pub cursor_active: bool,
    pub z: i32,
}

impl ClipRect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn clamp(self, size: (u32, u32)) -> Option<Self> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        let max_width = size.0 as i32;
        let max_height = size.1 as i32;
        let mut x = self.x;
        let mut y = self.y;
        let mut w = self.width as i32;
        let mut h = self.height as i32;
        if x >= max_width || y >= max_height {
            return None;
        }
        if x < 0 {
            w += x;
            x = 0;
        }
        if y < 0 {
            h += y;
            y = 0;
        }
        let right = (x + w).min(max_width);
        let bottom = (y + h).min(max_height);
        let width = (right - x).max(0);
        let height = (bottom - y).max(0);
        if width == 0 || height == 0 {
            return None;
        }
        Some(Self {
            x,
            y,
            width: width as u32,
            height: height as u32,
        })
    }

    pub fn intersect(&self, other: &ClipRect) -> Option<Self> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y2 = (self.y + self.height as i32).min(other.y + other.height as i32);
        if x2 <= x1 || y2 <= y1 {
            return None;
        }
        Some(Self {
            x: x1,
            y: y1,
            width: (x2 - x1) as u32,
            height: (y2 - y1) as u32,
        })
    }
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

#[derive(Clone)]
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
        clear_frame(frame);
        if let Some(data) = frame_data {
            if data.tilemaps.is_empty() {
                if let Some(fallback) = data.fallback {
                    draw_tile_scene(
                        fallback.scene,
                        fallback.camera,
                        fallback.player_marker.as_ref(),
                        self.logical_size,
                        frame,
                        frame_index,
                    );
                    draw_sprites(self.logical_size, frame, fallback.sprites);
                } else {
                    draw_gradient(self.logical_size, frame, frame_index);
                }
            } else {
                draw_tilemaps(self.logical_size, frame, data.tilemaps, frame_index);
            }
            draw_planes(self.logical_size, frame, data.planes);
            draw_sprites(self.logical_size, frame, data.sprites);
            draw_windows(self.logical_size, frame, data.windows);
        } else {
            draw_gradient(self.logical_size, frame, frame_index);
        }
        let effects = screen_effects();
        apply_screen_effects(self.logical_size, frame, &effects);
        capture_backbuffer(self.logical_size, frame);
        self.pixels.render()?;
        Ok(())
    }
}

fn clear_frame(frame: &mut [u8]) {
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 0, 0xFF]);
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
    let clip = ClipRect::new(0, 0, size.0, size.1);
    let neutral_tone = [0.0, 0.0, 0.0, 0.0];
    let neutral_color = [0.0, 0.0, 0.0, 0.0];
    draw_tilemap_region(
        scene,
        camera,
        clip,
        size,
        frame,
        frame_index,
        &neutral_tone,
        &neutral_color,
        255,
        0,
    );
    if let Some(marker) = player_marker {
        draw_player_marker(scene, camera, marker, size, frame);
    }
}

fn draw_tilemap_region(
    scene: &TileScene,
    camera: Camera,
    clip: ClipRect,
    size: (u32, u32),
    frame: &mut [u8],
    frame_index: u64,
    tone: &[f32; 4],
    overlay: &[f32; 4],
    opacity: u8,
    blend_type: u8,
) {
    let Some(clip) = clip.clamp(size) else {
        return;
    };
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
    for y in 0..clip.height as usize {
        let dest_y = clip.y as usize + y;
        if dest_y >= height {
            continue;
        }
        let world_y = cam_y + y as f32;
        let tile_y = (world_y as usize) / scene.tile_size;
        if tile_y >= scene.map_height {
            fill_background_line(frame, width, dest_y);
            continue;
        }
        let local_y = (world_y as usize) % scene.tile_size;
        for x in 0..clip.width as usize {
            let dest_x = clip.x as usize + x;
            if dest_x >= width {
                break;
            }
            let world_x = cam_x + x as f32;
            if world_x < 0.0 || world_x >= map_width_px as f32 {
                set_pixel(
                    frame,
                    width,
                    dest_x,
                    dest_y,
                    debug_pixel(dest_x, dest_y, width),
                );
                continue;
            }
            let tile_x = (world_x as usize) / scene.tile_size;
            if tile_x >= scene.map_width {
                set_pixel(
                    frame,
                    width,
                    dest_x,
                    dest_y,
                    debug_pixel(dest_x, dest_y, width),
                );
                continue;
            }
            let local_x = (world_x as usize) % scene.tile_size;
            let mut ground = [0, 0, 0, 0];
            let mut overlay_pixels = [0, 0, 0, 0];
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
                    blend_pixel(&mut overlay_pixels, sample);
                }
            }
            let mut final_color = ground;
            if overlay_pixels[3] > 0 {
                blend_pixel(&mut final_color, overlay_pixels);
            }
            if final_color[3] == 0 {
                final_color = debug_pixel(dest_x, dest_y, width);
            }
            final_color = apply_tone_and_color(final_color, tone, overlay);
            if opacity < 255 {
                final_color[3] = ((final_color[3] as u16 * opacity as u16) / 255) as u8;
            }
            let idx = (dest_y as usize * width + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_with_mode(&mut dst, final_color, blend_type);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn draw_tilemaps(
    size: (u32, u32),
    frame: &mut [u8],
    tilemaps: &[TilemapInstance],
    frame_index: u64,
) {
    for tilemap in tilemaps {
        draw_tilemap_region(
            &tilemap.scene,
            tilemap.camera,
            tilemap.clip,
            size,
            frame,
            frame_index,
            &tilemap.tone,
            &tilemap.color,
            tilemap.opacity,
            tilemap.blend_type,
        );
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

fn apply_tone_and_color(mut color: [u8; 4], tone: &[f32; 4], overlay: &[f32; 4]) -> [u8; 4] {
    let mut r = (color[0] as f32 + tone[0]).clamp(0.0, 255.0);
    let mut g = (color[1] as f32 + tone[1]).clamp(0.0, 255.0);
    let mut b = (color[2] as f32 + tone[2]).clamp(0.0, 255.0);
    let gray = tone[3].clamp(0.0, 255.0) / 255.0;
    if gray > 0.0 {
        let avg = (r + g + b) / 3.0;
        r = r + (avg - r) * gray;
        g = g + (avg - g) * gray;
        b = b + (avg - b) * gray;
    }
    let overlay_alpha = (overlay[3] / 255.0).clamp(0.0, 1.0);
    if overlay_alpha > 0.0 {
        r = r + (overlay[0].clamp(0.0, 255.0) - r) * overlay_alpha;
        g = g + (overlay[1].clamp(0.0, 255.0) - g) * overlay_alpha;
        b = b + (overlay[2].clamp(0.0, 255.0) - b) * overlay_alpha;
    }
    color[0] = r.clamp(0.0, 255.0) as u8;
    color[1] = g.clamp(0.0, 255.0) as u8;
    color[2] = b.clamp(0.0, 255.0) as u8;
    color
}

fn blend_with_mode(dst: &mut [u8; 4], src: [u8; 4], blend_type: u8) {
    match blend_type {
        1 => {
            let alpha = src[3] as f32 / 255.0;
            for i in 0..3 {
                let added = dst[i] as f32 + src[i] as f32 * alpha;
                dst[i] = added.min(255.0) as u8;
            }
            dst[3] = dst[3].max(src[3]);
        }
        2 => {
            let alpha = src[3] as f32 / 255.0;
            for i in 0..3 {
                let sub = (src[i] as f32 * alpha) as i32;
                let value = dst[i] as i32 - sub;
                dst[i] = value.max(0) as u8;
            }
        }
        _ => blend_pixel(dst, src),
    }
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
    let Some(clip) = sprite.clip.clamp(size) else {
        return;
    };
    if sprite.src_rect.2 == 0 || sprite.src_rect.3 == 0 {
        return;
    }
    let tex_width = sprite.texture.width();
    let tex_height = sprite.texture.height();
    let src_left = sprite.src_rect.0 as f32;
    let src_top = sprite.src_rect.1 as f32;
    let src_right = src_left + sprite.src_rect.2 as f32;
    let src_bottom = src_top + sprite.src_rect.3 as f32;
    let mut scale_x = if sprite.scale.0.abs() < std::f32::EPSILON {
        1.0
    } else {
        sprite.scale.0
    };
    let mut scale_y = if sprite.scale.1.abs() < std::f32::EPSILON {
        1.0
    } else {
        sprite.scale.1
    };
    if scale_x.abs() < 1.0e-3 {
        scale_x = 1.0;
    }
    if scale_y.abs() < 1.0e-3 {
        scale_y = 1.0;
    }
    let inv_scale_x = 1.0 / scale_x;
    let inv_scale_y = 1.0 / scale_y;
    let angle = sprite.angle.to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();
    let pivot_x = sprite.origin.0;
    let pivot_y = sprite.origin.1;
    for y in 0..clip.height {
        let dest_y = clip.y + y as i32;
        for x in 0..clip.width {
            let dest_x = clip.x + x as i32;
            let dx = dest_x as f32 - sprite.position.0;
            let dy = dest_y as f32 - sprite.position.1;
            let inv_x = cos_a * dx + sin_a * dy;
            let inv_y = -sin_a * dx + cos_a * dy;
            let mut local_x = inv_x * inv_scale_x;
            let local_y = inv_y * inv_scale_y;
            if sprite.mirror {
                local_x = -local_x;
            }
            let src_x = src_left + pivot_x + local_x;
            if src_x < src_left || src_x >= src_right {
                continue;
            }
            let src_y = src_top + pivot_y + local_y;
            if src_y < src_top || src_y >= src_bottom {
                continue;
            }
            let sample_x = src_x.floor() as u32;
            let sample_y = src_y.floor() as u32;
            if sample_x >= tex_width || sample_y >= tex_height {
                continue;
            }
            let mut color = sprite.texture.get_pixel(sample_x, sample_y).0;
            if color[3] == 0 {
                continue;
            }
            if sprite.opacity < 255 {
                color[3] = ((color[3] as u16 * sprite.opacity as u16) / 255) as u8;
            }
            if sprite.bush_depth > 0 {
                let local_y_px = src_y - src_top;
                if local_y_px >= sprite.src_rect.3 as f32 - sprite.bush_depth as f32 {
                    color[3] = ((color[3] as u16 * sprite.bush_opacity as u16) / 255) as u8;
                }
            }
            color = apply_tone_and_color(color, &sprite.tone, &sprite.color);
            let idx = (dest_y as usize * size.0 as usize + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_with_mode(&mut dst, color, sprite.blend_type);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn draw_planes(size: (u32, u32), frame: &mut [u8], planes: &[PlaneInstance]) {
    if planes.is_empty() {
        return;
    }
    for plane in planes {
        if let Some(clip) = plane.clip.clamp(size) {
            draw_plane_instance(size, frame, plane, clip);
        }
    }
}

fn draw_plane_instance(size: (u32, u32), frame: &mut [u8], plane: &PlaneInstance, clip: ClipRect) {
    let texture = &plane.texture;
    let tex_w = texture.width() as i32;
    let tex_h = texture.height() as i32;
    if tex_w == 0 || tex_h == 0 {
        return;
    }
    let zoom_x = plane.zoom.0.abs().max(0.001);
    let zoom_y = plane.zoom.1.abs().max(0.001);
    for y in 0..clip.height {
        let dest_y = clip.y + y as i32;
        let sample_y = ((plane.scroll.1 + y as f32) / zoom_y).floor() as i32;
        let wrapped_y = wrap_coord(sample_y, tex_h);
        for x in 0..clip.width {
            let dest_x = clip.x + x as i32;
            let sample_x = ((plane.scroll.0 + x as f32) / zoom_x).floor() as i32;
            let wrapped_x = wrap_coord(sample_x, tex_w);
            let mut color = texture.get_pixel(wrapped_x as u32, wrapped_y as u32).0;
            if color[3] == 0 {
                continue;
            }
            color[3] = ((color[3] as u16 * plane.opacity as u16) / 255) as u8;
            color = apply_tone_and_color(color, &plane.tone, &plane.color);
            let idx = (dest_y as usize * size.0 as usize + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_with_mode(&mut dst, color, plane.blend_type);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn draw_windows(size: (u32, u32), frame: &mut [u8], windows: &[WindowInstance]) {
    if windows.is_empty() {
        return;
    }
    for window in windows {
        draw_window(size, frame, window);
    }
}

fn draw_window(size: (u32, u32), frame: &mut [u8], window: &WindowInstance) {
    if window.opacity == 0 {
        return;
    }
    let Some(visible) = window
        .visible_rect
        .intersect(&window.clip)
        .and_then(|rect| rect.clamp(size))
    else {
        return;
    };
    if let Some(skin) = window.windowskin.as_ref() {
        draw_window_background(window, skin, &visible, size, frame);
        draw_window_frame(window, skin, &visible, size, frame);
    } else {
        fill_rect(size, frame, &visible, [0, 0, 0, window.opacity]);
    }
    if let Some(contents) = window.contents.as_ref() {
        draw_window_contents(window, contents, &visible, size, frame);
    }
    if window.cursor_active {
        draw_window_cursor(window, &visible, size, frame);
    }
}

fn draw_window_background(
    window: &WindowInstance,
    skin: &RgbaImage,
    visible: &ClipRect,
    size: (u32, u32),
    frame: &mut [u8],
) {
    let inner_width = window
        .frame_rect
        .width
        .saturating_sub((WINDOW_PADDING * 2) as u32);
    let inner_height = window
        .frame_rect
        .height
        .saturating_sub((WINDOW_PADDING * 2) as u32);
    if inner_width == 0 || inner_height == 0 {
        return;
    }
    let inner_rect = ClipRect::new(
        window.frame_rect.x + WINDOW_PADDING,
        window.frame_rect.y + WINDOW_PADDING,
        inner_width,
        inner_height,
    );
    let Some(target) = inner_rect.intersect(visible).and_then(|r| r.clamp(size)) else {
        return;
    };
    let base_opacity = ((window.opacity as u16 * window.back_opacity as u16) / 255).min(255) as u8;
    if base_opacity == 0 {
        return;
    }
    for y in 0..target.height {
        let dest_y = target.y + y as i32;
        let local_y = dest_y - inner_rect.y;
        let sample_y = WINDOW_BG_SRC.y + wrap_coord(local_y as i32, WINDOW_BG_SRC.h as i32) as u32;
        for x in 0..target.width {
            let dest_x = target.x + x as i32;
            let local_x = dest_x - inner_rect.x;
            let sample_x =
                WINDOW_BG_SRC.x + wrap_coord(local_x as i32, WINDOW_BG_SRC.w as i32) as u32;
            let mut color = skin.get_pixel(sample_x, sample_y).0;
            if color[3] == 0 {
                continue;
            }
            color[3] = ((color[3] as u16 * base_opacity as u16) / 255) as u8;
            color = apply_tone_and_color(color, &window.tone, &window.color);
            let idx = (dest_y as usize * size.0 as usize + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_pixel(&mut dst, color);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn draw_window_frame(
    window: &WindowInstance,
    skin: &RgbaImage,
    visible: &ClipRect,
    size: (u32, u32),
    frame: &mut [u8],
) {
    let opacity = window.opacity;
    if opacity == 0 {
        return;
    }
    let corner_size = WINDOW_PADDING.max(0) as u32;
    let full_rect = window.frame_rect;
    let corner_w = corner_size.min(full_rect.width);
    let corner_h = corner_size.min(full_rect.height);
    // Corners: top-left, top-right, bottom-left, bottom-right
    let corners = [
        ClipRect::new(full_rect.x, full_rect.y, corner_w, corner_h),
        ClipRect::new(
            full_rect.x + full_rect.width as i32 - corner_w as i32,
            full_rect.y,
            corner_w,
            corner_h,
        ),
        ClipRect::new(
            full_rect.x,
            full_rect.y + full_rect.height as i32 - corner_h as i32,
            corner_w,
            corner_h,
        ),
        ClipRect::new(
            full_rect.x + full_rect.width as i32 - corner_w as i32,
            full_rect.y + full_rect.height as i32 - corner_h as i32,
            corner_w,
            corner_h,
        ),
    ];
    for (dest, src) in corners.iter().zip(WINDOW_CORNER_SRC.iter()) {
        draw_windowskin_patch(
            skin,
            *src,
            dest,
            visible,
            size,
            frame,
            &window.tone,
            &window.color,
            opacity,
        );
    }
    // Edges: top, bottom, left, right
    let horizontal_width = full_rect.width.saturating_sub((WINDOW_PADDING * 2) as u32);
    if horizontal_width > 0 {
        let top = ClipRect::new(
            full_rect.x + WINDOW_PADDING,
            full_rect.y,
            horizontal_width,
            corner_h,
        );
        let bottom = ClipRect::new(
            full_rect.x + WINDOW_PADDING,
            full_rect.y + full_rect.height as i32 - corner_h as i32,
            horizontal_width,
            corner_h,
        );
        draw_windowskin_patch(
            skin,
            WINDOW_EDGE_SRC[0],
            &top,
            visible,
            size,
            frame,
            &window.tone,
            &window.color,
            opacity,
        );
        draw_windowskin_patch(
            skin,
            WINDOW_EDGE_SRC[1],
            &bottom,
            visible,
            size,
            frame,
            &window.tone,
            &window.color,
            opacity,
        );
    }
    let vertical_height = full_rect.height.saturating_sub((WINDOW_PADDING * 2) as u32);
    if vertical_height > 0 {
        let left = ClipRect::new(
            full_rect.x,
            full_rect.y + WINDOW_PADDING,
            corner_w,
            vertical_height,
        );
        let right = ClipRect::new(
            full_rect.x + full_rect.width as i32 - corner_w as i32,
            full_rect.y + WINDOW_PADDING,
            corner_w,
            vertical_height,
        );
        draw_windowskin_patch(
            skin,
            WINDOW_EDGE_SRC[2],
            &left,
            visible,
            size,
            frame,
            &window.tone,
            &window.color,
            opacity,
        );
        draw_windowskin_patch(
            skin,
            WINDOW_EDGE_SRC[3],
            &right,
            visible,
            size,
            frame,
            &window.tone,
            &window.color,
            opacity,
        );
    }
}

fn draw_window_contents(
    window: &WindowInstance,
    contents: &RgbaImage,
    visible: &ClipRect,
    size: (u32, u32),
    frame: &mut [u8],
) {
    let inner_width = window
        .frame_rect
        .width
        .saturating_sub((WINDOW_PADDING * 2) as u32);
    let inner_height = window
        .frame_rect
        .height
        .saturating_sub((WINDOW_PADDING * 2) as u32);
    if inner_width == 0 || inner_height == 0 {
        return;
    }
    let inner_rect = ClipRect::new(
        window.frame_rect.x + WINDOW_PADDING,
        window.frame_rect.y + WINDOW_PADDING,
        inner_width,
        inner_height,
    );
    let Some(target) = inner_rect.intersect(visible).and_then(|r| r.clamp(size)) else {
        return;
    };
    let base_opacity =
        ((window.opacity as u16 * window.contents_opacity as u16) / 255).min(255) as u8;
    if base_opacity == 0 {
        return;
    }
    let tex_w = contents.width() as i32;
    let tex_h = contents.height() as i32;
    for y in 0..target.height {
        let dest_y = target.y + y as i32;
        let src_y = dest_y - inner_rect.y + window.contents_origin.1;
        if src_y < 0 || src_y >= tex_h {
            continue;
        }
        for x in 0..target.width {
            let dest_x = target.x + x as i32;
            let src_x = dest_x - inner_rect.x + window.contents_origin.0;
            if src_x < 0 || src_x >= tex_w {
                continue;
            }
            let mut color = contents.get_pixel(src_x as u32, src_y as u32).0;
            if color[3] == 0 {
                continue;
            }
            color[3] = ((color[3] as u16 * base_opacity as u16) / 255) as u8;
            let idx = (dest_y as usize * size.0 as usize + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_pixel(&mut dst, color);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn draw_window_cursor(
    window: &WindowInstance,
    visible: &ClipRect,
    size: (u32, u32),
    frame: &mut [u8],
) {
    let Some(cursor_rect) = window.cursor_rect.as_ref() else {
        return;
    };
    let Some(dest) = cursor_rect.intersect(visible).and_then(|r| r.clamp(size)) else {
        return;
    };
    if dest.width == 0 || dest.height == 0 {
        return;
    }
    if let Some(skin) = window.windowskin.as_ref() {
        draw_windowskin_patch(
            skin,
            WINDOW_CURSOR_SRC,
            &dest,
            visible,
            size,
            frame,
            &window.tone,
            &window.color,
            window.opacity,
        );
    } else {
        fill_rect(size, frame, &dest, [255, 255, 255, window.opacity]);
    }
}

fn draw_windowskin_patch(
    skin: &RgbaImage,
    src: SkinRect,
    dest: &ClipRect,
    visible: &ClipRect,
    size: (u32, u32),
    frame: &mut [u8],
    tone: &[f32; 4],
    color: &[f32; 4],
    opacity: u8,
) {
    if src.w == 0 || src.h == 0 || dest.width == 0 || dest.height == 0 || opacity == 0 {
        return;
    }
    let Some(target) = dest.intersect(visible).and_then(|r| r.clamp(size)) else {
        return;
    };
    for y in 0..target.height {
        let dest_y = target.y + y as i32;
        let ty = src.y
            + (((dest_y - dest.y) as f32 / dest.height as f32) * (src.h as f32 - 1.0))
                .clamp(0.0, src.h as f32 - 1.0) as u32;
        for x in 0..target.width {
            let dest_x = target.x + x as i32;
            let tx = src.x
                + (((dest_x - dest.x) as f32 / dest.width as f32) * (src.w as f32 - 1.0))
                    .clamp(0.0, src.w as f32 - 1.0) as u32;
            let mut sample = skin.get_pixel(tx, ty).0;
            if sample[3] == 0 {
                continue;
            }
            sample[3] = ((sample[3] as u16 * opacity as u16) / 255) as u8;
            sample = apply_tone_and_color(sample, tone, color);
            let idx = (dest_y as usize * size.0 as usize + dest_x as usize) * 4;
            let mut dst = [frame[idx], frame[idx + 1], frame[idx + 2], frame[idx + 3]];
            blend_pixel(&mut dst, sample);
            frame[idx..idx + 4].copy_from_slice(&dst);
        }
    }
}

fn fill_rect(size: (u32, u32), frame: &mut [u8], rect: &ClipRect, color: [u8; 4]) {
    for y in 0..rect.height {
        let dest_y = rect.y + y as i32;
        for x in 0..rect.width {
            let dest_x = rect.x + x as i32;
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

fn capture_backbuffer(size: (u32, u32), frame: &[u8]) {
    if let Some(image) = RgbaImage::from_raw(size.0, size.1, frame.to_vec()) {
        store_backbuffer(Arc::new(image));
    }
}

fn apply_screen_effects(_size: (u32, u32), frame: &mut [u8], effects: &ScreenEffects) {
    let mut apply_tone = false;
    for value in effects.tone.iter() {
        if value.abs() > f32::EPSILON {
            apply_tone = true;
            break;
        }
    }
    let apply_brightness = (effects.brightness - 1.0).abs() > f32::EPSILON;
    let apply_flash = effects.flash.is_some();
    if !apply_tone && !apply_brightness && !apply_flash {
        return;
    }
    let flash_color = effects.flash.map(|color| {
        [
            color[0].clamp(0.0, 255.0) as u8,
            color[1].clamp(0.0, 255.0) as u8,
            color[2].clamp(0.0, 255.0) as u8,
            color[3].clamp(0.0, 255.0) as u8,
        ]
    });
    for pixel in frame.chunks_exact_mut(4) {
        let mut rgb = [pixel[0] as f32, pixel[1] as f32, pixel[2] as f32];
        if apply_brightness {
            for channel in rgb.iter_mut() {
                *channel = (*channel * effects.brightness).clamp(0.0, 255.0);
            }
        }
        if apply_tone {
            rgb[0] = (rgb[0] + effects.tone[0]).clamp(0.0, 255.0);
            rgb[1] = (rgb[1] + effects.tone[1]).clamp(0.0, 255.0);
            rgb[2] = (rgb[2] + effects.tone[2]).clamp(0.0, 255.0);
            let gray = effects.tone[3].clamp(0.0, 255.0) / 255.0;
            if gray > 0.0 {
                let average = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
                for channel in rgb.iter_mut() {
                    *channel = (*channel + (average - *channel) * gray).clamp(0.0, 255.0);
                }
            }
        }
        pixel[0] = rgb[0] as u8;
        pixel[1] = rgb[1] as u8;
        pixel[2] = rgb[2] as u8;
        if let Some(color) = flash_color {
            let mut dst = [pixel[0], pixel[1], pixel[2], pixel[3]];
            blend_pixel(&mut dst, color);
            pixel[0..3].copy_from_slice(&dst[0..3]);
            pixel[3] = dst[3];
        }
    }
}

fn wrap_coord(value: i32, max: i32) -> i32 {
    if max == 0 {
        return 0;
    }
    let mut result = value % max;
    if result < 0 {
        result += max;
    }
    result
}
