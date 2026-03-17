use anyhow::Result;
use pixels::{Pixels, SurfaceTexture};
use winit::{dpi::PhysicalSize, window::Window};

/// Basic renderer responsible for presenting frames using `pixels`.
pub struct Renderer<'a> {
    pixels: Pixels<'a>,
    logical_size: (u32, u32),
}

#[derive(Debug, Clone)]
pub struct TileDebugView {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<i16>,
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

    pub fn render(&mut self, frame_index: u64, view: Option<&TileDebugView>) -> Result<()> {
        let frame = self.pixels.frame_mut();
        match view {
            Some(debug) => draw_tile_debug(debug, self.logical_size, frame),
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

fn draw_tile_debug(view: &TileDebugView, size: (u32, u32), frame: &mut [u8]) {
    let width = size.0 as usize;
    let height = size.1 as usize;
    for y in 0..height {
        let tile_y = y * view.height / height;
        for x in 0..width {
            let tile_x = x * view.width / width;
            let idx = tile_y * view.width + tile_x;
            let tile_id = *view.tiles.get(idx).unwrap_or(&0);
            let color = color_for_tile(tile_id);
            let offset = (y * width + x) * 4;
            frame[offset] = color[0];
            frame[offset + 1] = color[1];
            frame[offset + 2] = color[2];
            frame[offset + 3] = 0xFF;
        }
    }
}

fn color_for_tile(tile_id: i16) -> [u8; 3] {
    let v = tile_id as i32;
    let r = v.wrapping_mul(97) as u8;
    let g = v.wrapping_mul(57) as u8;
    let b = v.wrapping_mul(31) as u8;
    [r, g, b]
}
