use anyhow::Result;
use pixels::{Pixels, SurfaceTexture};
use winit::{dpi::PhysicalSize, window::Window};

/// Basic renderer responsible for presenting frames using `pixels`.
pub struct Renderer<'a> {
    pixels: Pixels<'a>,
    logical_size: (u32, u32),
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

    pub fn render(&mut self, frame_index: u64) -> Result<()> {
        let frame = self.pixels.frame_mut();
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % self.logical_size.0 as usize) as u8;
            let y = (i / self.logical_size.0 as usize) as u8;
            let t = frame_index as u8;
            pixel[0] = x.wrapping_add(t);
            pixel[1] = y.wrapping_add(t);
            pixel[2] = 0x80;
            pixel[3] = 0xFF;
        }
        self.pixels.render()?;
        Ok(())
    }
}
