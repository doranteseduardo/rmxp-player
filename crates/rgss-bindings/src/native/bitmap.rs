use super::{module, native_module, HandleStore};
use anyhow::{Context, Result};
use image::{ImageReader, Rgba, RgbaImage};
use once_cell::sync::Lazy;
use rb_sys::{rb_num2int, rb_num2uint, rb_uint2inum, ruby_special_consts, special_consts, VALUE};
use std::{
    ffi::CStr,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{info, warn};

const BITMAP_CREATE_NAME: &[u8] = b"bitmap_create\0";
const BITMAP_DISPOSE_NAME: &[u8] = b"bitmap_dispose\0";
const BITMAP_WIDTH_NAME: &[u8] = b"bitmap_width\0";
const BITMAP_HEIGHT_NAME: &[u8] = b"bitmap_height\0";
const BITMAP_DISPOSED_NAME: &[u8] = b"bitmap_disposed?\0";
const BITMAP_LOAD_NAME: &[u8] = b"bitmap_load\0";
const BITMAP_FILL_RECT_NAME: &[u8] = b"bitmap_fill_rect\0";
const BITMAP_CLEAR_NAME: &[u8] = b"bitmap_clear\0";
const BITMAP_GET_PIXEL_NAME: &[u8] = b"bitmap_get_pixel\0";
const BITMAP_SET_PIXEL_NAME: &[u8] = b"bitmap_set_pixel\0";
const BITMAP_BLT_NAME: &[u8] = b"bitmap_blt\0";

static BITMAPS: Lazy<HandleStore<BitmapData>> = Lazy::new(HandleStore::default);

#[derive(Clone, Debug)]
pub struct BitmapData {
    pub width: u32,
    pub height: u32,
    pub disposed: bool,
    pub texture: Arc<RgbaImage>,
}

impl BitmapData {
    fn blank(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let texture = Arc::new(RgbaImage::from_pixel(
            width,
            height,
            image::Rgba([0, 0, 0, 0]),
        ));
        Self {
            width,
            height,
            disposed: false,
            texture,
        }
    }

    fn with_texture(texture: Arc<RgbaImage>) -> Self {
        let width = texture.width();
        let height = texture.height();
        Self {
            width,
            height,
            disposed: false,
            texture,
        }
    }
}

extern "C" {
    fn rb_define_module_function(
        module: VALUE,
        name: *const c_char,
        func: Option<unsafe extern "C" fn(c_int, *const VALUE, VALUE) -> VALUE>,
        argc: c_int,
    );
    fn rb_string_value_cstr(ptr: *mut VALUE) -> *const c_char;
}

pub fn init() -> Result<()> {
    unsafe { define_bitmap_api() }
}

pub fn snapshot() -> Vec<(u32, BitmapData)> {
    BITMAPS.snapshot()
}

pub fn create_from_texture(texture: Arc<RgbaImage>) -> u32 {
    store_bitmap_data(BitmapData::with_texture(texture))
}

unsafe fn define_bitmap_api() -> Result<()> {
    let native = native_module()?;
    rb_define_module_function(native, c_name(BITMAP_CREATE_NAME), Some(bitmap_create), 2);
    rb_define_module_function(native, c_name(BITMAP_DISPOSE_NAME), Some(bitmap_dispose), 1);
    rb_define_module_function(native, c_name(BITMAP_WIDTH_NAME), Some(bitmap_width), 1);
    rb_define_module_function(native, c_name(BITMAP_HEIGHT_NAME), Some(bitmap_height), 1);
    rb_define_module_function(
        native,
        c_name(BITMAP_DISPOSED_NAME),
        Some(bitmap_disposed_q),
        1,
    );
    rb_define_module_function(native, c_name(BITMAP_LOAD_NAME), Some(bitmap_load), 1);
    rb_define_module_function(
        native,
        c_name(BITMAP_FILL_RECT_NAME),
        Some(bitmap_fill_rect),
        6,
    );
    rb_define_module_function(native, c_name(BITMAP_CLEAR_NAME), Some(bitmap_clear), 1);
    rb_define_module_function(
        native,
        c_name(BITMAP_GET_PIXEL_NAME),
        Some(bitmap_get_pixel),
        3,
    );
    rb_define_module_function(
        native,
        c_name(BITMAP_SET_PIXEL_NAME),
        Some(bitmap_set_pixel),
        -1,
    );
    rb_define_module_function(native, c_name(BITMAP_BLT_NAME), Some(bitmap_blt), -1);
    Ok(())
}

unsafe extern "C" fn bitmap_create(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 1 || argv.is_null() {
        warn!(target: "rgss", "bitmap_create missing arguments");
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let width = clamp_dimension(rb_num2int(args[0]) as i32);
    let height = if args.len() >= 2 {
        clamp_dimension(rb_num2int(args[1]) as i32)
    } else {
        width
    };
    let id = store_bitmap(width, height);
    rb_uint2inum(id as usize)
}

unsafe extern "C" fn bitmap_load(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let mut arg = *argv;
    let path_ptr = rb_string_value_cstr(&mut arg);
    if path_ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let path = CStr::from_ptr(path_ptr).to_string_lossy().to_string();
    match load_bitmap_from_project(&path) {
        Ok(id) => rb_uint2inum(id as usize),
        Err(err) => {
            warn!(target: "rgss", path = %path, error = %err, "Failed to load bitmap");
            rb_sys::Qnil as VALUE
        }
    }
}

unsafe extern "C" fn bitmap_dispose(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    BITMAPS.with_mut(id, |entry| {
        entry.disposed = true;
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_width(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let width = with_bitmap(id, |entry| entry.width).unwrap_or(0);
    int_to_value(width as i64)
}

unsafe extern "C" fn bitmap_height(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let height = with_bitmap(id, |entry| entry.height).unwrap_or(0);
    int_to_value(height as i64)
}

unsafe extern "C" fn bitmap_disposed_q(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qfalse as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let disposed = with_bitmap(id, |entry| entry.disposed).unwrap_or(true);
    if disposed {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

fn store_bitmap(width: u32, height: u32) -> u32 {
    store_bitmap_data(BitmapData::blank(width, height))
}

fn with_bitmap<F, R>(id: u32, func: F) -> Option<R>
where
    F: FnOnce(&BitmapData) -> R,
{
    BITMAPS.with(id, func)
}

fn clamp_dimension(value: i32) -> u32 {
    value.max(1) as u32
}

fn int_to_value(value: i64) -> VALUE {
    unsafe {
        if value >= special_consts::FIXNUM_MIN as i64 && value <= special_consts::FIXNUM_MAX as i64
        {
            ((value << ruby_special_consts::RUBY_SPECIAL_SHIFT as i64)
                | ruby_special_consts::RUBY_FIXNUM_FLAG as i64) as VALUE
        } else {
            rb_sys::rb_int2big(value as isize)
        }
    }
}

fn c_name(bytes: &[u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}

fn store_bitmap_data(data: BitmapData) -> u32 {
    BITMAPS.insert(data)
}

fn load_bitmap_from_project(relative: &str) -> Result<u32> {
    let root = module::project_root().ok_or_else(|| anyhow::anyhow!("project root not set"))?;
    let candidates = candidate_paths(relative);
    for candidate in candidates {
        let path = if candidate.is_absolute() {
            candidate
        } else {
            root.join(&candidate)
        };
        if path.exists() {
            return load_bitmap_from_file(&path);
        }
    }
    anyhow::bail!("{} not found under {}", relative, root.display())
}

fn candidate_paths(relative: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let base = PathBuf::from(relative);
    paths.push(base.clone());
    if base.extension().is_none() {
        paths.push(base.with_extension("png"));
        paths.push(base.with_extension("PNG"));
    }
    paths
}

fn load_bitmap_from_file(path: &Path) -> Result<u32> {
    let image = ImageReader::open(path)
        .with_context(|| format!("opening {}", path.display()))?
        .decode()
        .with_context(|| format!("decoding {}", path.display()))?
        .to_rgba8();
    info!(
        target: "rgss",
        file = %path.display(),
        width = image.width(),
        height = image.height(),
        "bitmap loaded"
    );
    Ok(store_bitmap_data(BitmapData::with_texture(Arc::new(image))))
}

fn with_bitmap_mut<F, R>(id: u32, func: F) -> Option<R>
where
    F: FnOnce(&mut RgbaImage) -> R,
{
    BITMAPS
        .with_mut(id, |entry| {
            if entry.disposed {
                return None;
            }
            if Arc::get_mut(&mut entry.texture).is_none() {
                let cloned = (*entry.texture).clone();
                entry.texture = Arc::new(cloned);
            }
            Arc::get_mut(&mut entry.texture).map(func)
        })
        .flatten()
}

fn bitmap_bounds(entry: &BitmapData) -> (u32, u32) {
    (entry.width.max(1), entry.height.max(1))
}

unsafe extern "C" fn bitmap_fill_rect(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 6 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let id = rb_num2uint(args[0]) as u32;
    let x = rb_num2int(args[1]) as i32;
    let y = rb_num2int(args[2]) as i32;
    let width = rb_num2int(args[3]).max(0) as i32;
    let height = rb_num2int(args[4]).max(0) as i32;
    let color = rb_num2uint(args[5]) as u32;
    let color = unpack_color(color);
    let _ = with_bitmap_mut(id, |image| {
        fill_rect_raw(image, x, y, width, height, color);
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_clear(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let _ = with_bitmap_mut(id, |image| {
        for pixel in image.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_get_pixel(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 3 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 3);
    let id = rb_num2uint(args[0]) as u32;
    let x = rb_num2int(args[1]) as i32;
    let y = rb_num2int(args[2]) as i32;
    let value = BITMAPS
        .with(id, |entry| {
            if entry.disposed {
                return None;
            }
            if x < 0 || y < 0 {
                return None;
            }
            let (width, height) = bitmap_bounds(entry);
            if x as u32 >= width || y as u32 >= height {
                return None;
            }
            let pixel = entry.texture.get_pixel(x as u32, y as u32);
            Some(pack_color(pixel.0))
        })
        .flatten();
    if let Some(color) = value {
        rb_uint2inum(color as usize)
    } else {
        rb_sys::Qnil as VALUE
    }
}

unsafe extern "C" fn bitmap_set_pixel(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 6 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let id = rb_num2uint(args[0]) as u32;
    let x = rb_num2int(args[1]);
    let y = rb_num2int(args[2]);
    let r = rb_num2int(args[3]) as i32;
    let g = rb_num2int(args[4]) as i32;
    let b = rb_num2int(args[5]) as i32;
    let a = if args.len() >= 7 {
        rb_num2int(args[6]) as i32
    } else {
        255
    };
    let color = [
        r.clamp(0, 255) as u8,
        g.clamp(0, 255) as u8,
        b.clamp(0, 255) as u8,
        a.clamp(0, 255) as u8,
    ];
    let _ = with_bitmap_mut(id, |image| {
        if x >= 0 && y >= 0 {
            let (width, height) = (image.width(), image.height());
            if (x as u32) < width && (y as u32) < height {
                let pixel = image.get_pixel_mut(x as u32, y as u32);
                *pixel = Rgba(color);
            }
        }
    });
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_blt(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 8 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let dest_id = rb_num2uint(args[0]) as u32;
    let dx = rb_num2int(args[1]) as i32;
    let dy = rb_num2int(args[2]) as i32;
    let src_id = rb_num2uint(args[3]) as u32;
    let sx = rb_num2int(args[4]) as i32;
    let sy = rb_num2int(args[5]) as i32;
    let sw = rb_num2int(args[6]).max(0) as i32;
    let sh = rb_num2int(args[7]).max(0) as i32;
    let opacity = if argc >= 9 {
        rb_num2int(args[8]).clamp(0, 255) as u8
    } else {
        255
    };

    let source = BITMAPS
        .with(src_id, |entry| {
            if entry.disposed {
                None
            } else {
                Some(entry.texture.clone())
            }
        })
        .flatten();
    let Some(source_image) = source else {
        return rb_sys::Qnil as VALUE;
    };
    let _ = with_bitmap_mut(dest_id, |dest_image| {
        blit_images(dest_image, dx, dy, &source_image, sx, sy, sw, sh, opacity);
    });
    rb_sys::Qnil as VALUE
}

fn fill_rect_raw(image: &mut RgbaImage, x: i32, y: i32, width: i32, height: i32, color: Rgba<u8>) {
    if width <= 0 || height <= 0 {
        return;
    }
    let width_u = image.width();
    let height_u = image.height();
    for yy in 0..height {
        let py = y + yy;
        if py < 0 || py as u32 >= height_u {
            continue;
        }
        for xx in 0..width {
            let px = x + xx;
            if px < 0 || px as u32 >= width_u {
                continue;
            }
            let mut dst = image.get_pixel_mut(px as u32, py as u32).0;
            blend_rgba(&mut dst, color.0);
            *image.get_pixel_mut(px as u32, py as u32) = Rgba(dst);
        }
    }
}

fn blit_images(
    dest: &mut RgbaImage,
    dx: i32,
    dy: i32,
    source: &Arc<RgbaImage>,
    sx: i32,
    sy: i32,
    sw: i32,
    sh: i32,
    opacity: u8,
) {
    if sw <= 0 || sh <= 0 {
        return;
    }
    let src_width = source.width() as i32;
    let src_height = source.height() as i32;
    for row in 0..sh {
        let src_y = sy + row;
        let dst_y = dy + row;
        if src_y < 0 || src_y >= src_height || dst_y < 0 || dst_y >= dest.height() as i32 {
            continue;
        }
        for col in 0..sw {
            let src_x = sx + col;
            let dst_x = dx + col;
            if src_x < 0 || src_x >= src_width || dst_x < 0 || dst_x >= dest.width() as i32 {
                continue;
            }
            let mut src_pixel = source.get_pixel(src_x as u32, src_y as u32).0;
            if opacity < 255 {
                src_pixel[3] = ((src_pixel[3] as u16 * opacity as u16) / 255) as u8;
            }
            if src_pixel[3] == 0 {
                continue;
            }
            let mut dst_pixel = dest.get_pixel(dst_x as u32, dst_y as u32).0;
            blend_rgba(&mut dst_pixel, src_pixel);
            *dest.get_pixel_mut(dst_x as u32, dst_y as u32) = Rgba(dst_pixel);
        }
    }
}

fn blend_rgba(dst: &mut [u8; 4], src: [u8; 4]) {
    let src_a = src[3] as f32 / 255.0;
    if src_a <= 0.0 {
        return;
    }
    if dst[3] == 0 && src[3] == 255 {
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

fn pack_color(pixel: [u8; 4]) -> u32 {
    (pixel[3] as u32) << 24 | (pixel[2] as u32) << 16 | (pixel[1] as u32) << 8 | pixel[0] as u32
}

fn unpack_color(value: u32) -> Rgba<u8> {
    let r = (value & 0xFF) as u8;
    let g = ((value >> 8) & 0xFF) as u8;
    let b = ((value >> 16) & 0xFF) as u8;
    let a = ((value >> 24) & 0xFF) as u8;
    Rgba([r, g, b, a])
}
