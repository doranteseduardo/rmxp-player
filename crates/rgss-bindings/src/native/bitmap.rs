use super::{module, native_module, value_to_bool, ColorData, HandleStore, RectData};
use crate::fs;
use anyhow::{Context, Result};
use font8x8::legacy::BASIC_LEGACY;
use image::{imageops::FilterType, ImageReader, Rgba, RgbaImage};
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
const BITMAP_GRADIENT_FILL_NAME: &[u8] = b"bitmap_gradient_fill_rect\0";
const BITMAP_STRETCH_BLT_NAME: &[u8] = b"bitmap_stretch_blt\0";
const BITMAP_DRAW_TEXT_NAME: &[u8] = b"bitmap_draw_text\0";
const BITMAP_TEXT_SIZE_NAME: &[u8] = b"bitmap_text_size\0";
const BITMAP_HUE_CHANGE_NAME: &[u8] = b"bitmap_hue_change\0";

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

pub fn create_blank(width: u32, height: u32) -> u32 {
    store_bitmap(width, height)
}

pub fn load_relative(path: &str) -> Result<u32> {
    load_bitmap_from_project(path)
}

pub fn dispose(id: u32) {
    BITMAPS.with_mut(id, |entry| {
        entry.disposed = true;
    });
}

pub fn is_disposed(id: u32) -> bool {
    BITMAPS.with(id, |entry| entry.disposed).unwrap_or(true)
}

pub fn dimensions(id: u32) -> Option<(u32, u32)> {
    BITMAPS
        .with(id, |entry| {
            if entry.disposed {
                None
            } else {
                Some((entry.width, entry.height))
            }
        })
        .flatten()
}

pub fn clear(id: u32) {
    let _ = with_bitmap_mut(id, |image| {
        for pixel in image.pixels_mut() {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    });
}

pub fn fill_rect(id: u32, rect: RectData, color: ColorData) {
    let color = color_to_rgba(color);
    let _ = with_bitmap_mut(id, |image| {
        fill_rect_raw(image, rect.x, rect.y, rect.width, rect.height, color);
    });
}

pub fn gradient_fill_rect(
    id: u32,
    rect: RectData,
    start: ColorData,
    end: ColorData,
    vertical: bool,
) {
    let start = color_to_rgba(start);
    let end = color_to_rgba(end);
    let _ = with_bitmap_mut(id, |image| {
        gradient_fill_raw(
            image,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            start,
            end,
            vertical,
        );
    });
}

pub fn blt(dest_id: u32, dx: i32, dy: i32, src_id: u32, src_rect: RectData, opacity: u8) {
    if let Some(source) = bitmap_texture(src_id) {
        let _ = with_bitmap_mut(dest_id, |dest_image| {
            blit_images(
                dest_image,
                dx,
                dy,
                &source,
                src_rect.x,
                src_rect.y,
                src_rect.width,
                src_rect.height,
                opacity,
            );
        });
    }
}

pub fn stretch_blt(
    dest_id: u32,
    dest_rect: RectData,
    src_id: u32,
    src_rect: RectData,
    opacity: u8,
) {
    if dest_rect.width <= 0 || dest_rect.height <= 0 || src_rect.width <= 0 || src_rect.height <= 0
    {
        return;
    }
    if let Some(source) = bitmap_texture(src_id) {
        if let Some(region) = extract_region(
            &source,
            src_rect.x,
            src_rect.y,
            src_rect.width,
            src_rect.height,
        ) {
            let scaled = image::imageops::resize(
                &region,
                dest_rect.width as u32,
                dest_rect.height as u32,
                FilterType::Triangle,
            );
            let scaled_arc = Arc::new(scaled);
            let _ = with_bitmap_mut(dest_id, |dest_image| {
                blit_images(
                    dest_image,
                    dest_rect.x,
                    dest_rect.y,
                    &scaled_arc,
                    0,
                    0,
                    dest_rect.width,
                    dest_rect.height,
                    opacity,
                );
            });
        }
    }
}

pub fn draw_text(
    id: u32,
    rect: RectData,
    text: &str,
    align: i32,
    font_size: i32,
    color: ColorData,
) {
    let font_size = font_size.max(6);
    let color = color_to_rgba(color);
    let _ = with_bitmap_mut(id, |image| {
        draw_text_run(
            image,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            text,
            align,
            font_size,
            color,
        );
    });
}

pub fn text_size(font_size: i32, text: &str) -> (i32, i32) {
    measure_text(text, font_size.max(6))
}

pub fn set_pixel(id: u32, x: i32, y: i32, color: ColorData) {
    let rgba = color_to_rgba(color);
    let _ = with_bitmap_mut(id, |image| {
        if x >= 0 && y >= 0 {
            let (width, height) = (image.width(), image.height());
            if (x as u32) < width && (y as u32) < height {
                let mut pixel = image.get_pixel_mut(x as u32, y as u32).0;
                blend_rgba(&mut pixel, rgba.0);
                *image.get_pixel_mut(x as u32, y as u32) = Rgba(pixel);
            }
        }
    });
}

pub fn get_pixel(id: u32, x: i32, y: i32) -> Option<ColorData> {
    BITMAPS
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
            Some(color_from_rgba(pixel.0))
        })
        .flatten()
}

pub fn copy_bitmap(src_id: u32) -> Option<u32> {
    BITMAPS
        .with(src_id, |entry| {
            if entry.disposed {
                None
            } else {
                Some(store_bitmap_data(BitmapData::with_texture(
                    entry.texture.clone(),
                )))
            }
        })
        .flatten()
}

fn bitmap_texture(id: u32) -> Option<Arc<RgbaImage>> {
    BITMAPS
        .with(id, |entry| {
            if entry.disposed {
                None
            } else {
                Some(entry.texture.clone())
            }
        })
        .flatten()
}

pub fn hue_change(id: u32, hue: i32) {
    let hue = normalize_hue(hue);
    if hue.abs() < f32::EPSILON {
        return;
    }
    let _ = with_bitmap_mut(id, |image| {
        for pixel in image.pixels_mut() {
            if pixel[3] == 0 {
                continue;
            }
            let (r, g, b) = (
                pixel[0] as f32 / 255.0,
                pixel[1] as f32 / 255.0,
                pixel[2] as f32 / 255.0,
            );
            let (mut h, s, v) = rgb_to_hsv(r, g, b);
            h += hue;
            while h < 0.0 {
                h += 360.0;
            }
            while h >= 360.0 {
                h -= 360.0;
            }
            let (nr, ng, nb) = hsv_to_rgb(h, s, v);
            pixel[0] = (nr * 255.0).round().clamp(0.0, 255.0) as u8;
            pixel[1] = (ng * 255.0).round().clamp(0.0, 255.0) as u8;
            pixel[2] = (nb * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    });
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
    rb_define_module_function(
        native,
        c_name(BITMAP_GRADIENT_FILL_NAME),
        Some(bitmap_gradient_fill_rect),
        -1,
    );
    rb_define_module_function(
        native,
        c_name(BITMAP_STRETCH_BLT_NAME),
        Some(bitmap_stretch_blt),
        -1,
    );
    rb_define_module_function(
        native,
        c_name(BITMAP_DRAW_TEXT_NAME),
        Some(bitmap_draw_text),
        -1,
    );
    rb_define_module_function(
        native,
        c_name(BITMAP_TEXT_SIZE_NAME),
        Some(bitmap_text_size),
        2,
    );
    rb_define_module_function(
        native,
        c_name(BITMAP_HUE_CHANGE_NAME),
        Some(bitmap_hue_change),
        2,
    );
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
    let id = create_blank(width, height);
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
    match load_relative(&path) {
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
    dispose(id);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_width(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let width = dimensions(id).map(|(w, _)| w).unwrap_or(0);
    int_to_value(width as i64)
}

unsafe extern "C" fn bitmap_height(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let height = dimensions(id).map(|(_, h)| h).unwrap_or(0);
    int_to_value(height as i64)
}

unsafe extern "C" fn bitmap_disposed_q(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qfalse as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    let disposed = is_disposed(id);
    if disposed {
        rb_sys::Qtrue as VALUE
    } else {
        rb_sys::Qfalse as VALUE
    }
}

fn store_bitmap(width: u32, height: u32) -> u32 {
    store_bitmap_data(BitmapData::blank(width, height))
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
    let candidates = candidate_paths(relative);
    for candidate in candidates {
        if let Some(path) = resolve_candidate(&candidate) {
            return load_bitmap_from_file(&path);
        }
    }
    let base = module::project_root()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<unknown>".into());
    anyhow::bail!("{} not found under {}", relative, base)
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

fn resolve_candidate(candidate: &Path) -> Option<PathBuf> {
    if candidate.is_absolute() {
        return candidate.exists().then(|| candidate.to_path_buf());
    }
    fs::resolve(candidate)
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
    let rect = RectData::new(
        rb_num2int(args[1]) as i32,
        rb_num2int(args[2]) as i32,
        rb_num2int(args[3]).max(0) as i32,
        rb_num2int(args[4]).max(0) as i32,
    );
    let packed = rb_num2uint(args[5]) as u32;
    let color = unpack_color(packed);
    fill_rect(
        id,
        rect,
        ColorData::new(
            color.0[0] as f32,
            color.0[1] as f32,
            color.0[2] as f32,
            color.0[3] as f32,
        ),
    );
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_gradient_fill_rect(
    argc: c_int,
    argv: *const VALUE,
    _self: VALUE,
) -> VALUE {
    if argc < 8 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let id = rb_num2uint(args[0]) as u32;
    let rect = RectData::new(
        rb_num2int(args[1]) as i32,
        rb_num2int(args[2]) as i32,
        rb_num2int(args[3]).max(0) as i32,
        rb_num2int(args[4]).max(0) as i32,
    );
    let color1 = unpack_color(rb_num2uint(args[5]) as u32);
    let color2 = unpack_color(rb_num2uint(args[6]) as u32);
    let vertical = value_to_bool(args[7]);
    gradient_fill_rect(
        id,
        rect,
        ColorData::new(
            color1.0[0] as f32,
            color1.0[1] as f32,
            color1.0[2] as f32,
            color1.0[3] as f32,
        ),
        ColorData::new(
            color2.0[0] as f32,
            color2.0[1] as f32,
            color2.0[2] as f32,
            color2.0[3] as f32,
        ),
        vertical,
    );
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_clear(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 1 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let id = rb_num2uint(*argv) as u32;
    clear(id);
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
    if let Some(color) = get_pixel(id, x, y) {
        let packed = pack_color([
            color.red as u8,
            color.green as u8,
            color.blue as u8,
            color.alpha as u8,
        ]);
        rb_uint2inum(packed as usize)
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
    let x = rb_num2int(args[1]) as i32;
    let y = rb_num2int(args[2]) as i32;
    let r = rb_num2int(args[3]) as i32;
    let g = rb_num2int(args[4]) as i32;
    let b = rb_num2int(args[5]) as i32;
    let a = if args.len() >= 7 {
        rb_num2int(args[6]) as i32
    } else {
        255
    };
    set_pixel(
        id,
        x,
        y,
        ColorData::new(
            r.clamp(0, 255) as f32,
            g.clamp(0, 255) as f32,
            b.clamp(0, 255) as f32,
            a.clamp(0, 255) as f32,
        ),
    );
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

    let rect = RectData::new(sx, sy, sw, sh);
    blt(dest_id, dx, dy, src_id, rect, opacity);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_stretch_blt(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 10 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let dest_id = rb_num2uint(args[0]) as u32;
    let dx = rb_num2int(args[1]) as i32;
    let dy = rb_num2int(args[2]) as i32;
    let dw = rb_num2int(args[3]).max(0) as i32;
    let dh = rb_num2int(args[4]).max(0) as i32;
    let src_id = rb_num2uint(args[5]) as u32;
    let sx = rb_num2int(args[6]) as i32;
    let sy = rb_num2int(args[7]) as i32;
    let sw = rb_num2int(args[8]).max(0) as i32;
    let sh = rb_num2int(args[9]).max(0) as i32;
    let opacity = if argc >= 11 {
        rb_num2int(args[10]).clamp(0, 255) as u8
    } else {
        255
    };
    if dw <= 0 || dh <= 0 || sw <= 0 || sh <= 0 {
        return rb_sys::Qnil as VALUE;
    }
    let dest_rect = RectData::new(dx, dy, dw, dh);
    let src_rect = RectData::new(sx, sy, sw, sh);
    stretch_blt(dest_id, dest_rect, src_id, src_rect, opacity);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_draw_text(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc < 9 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, argc as usize);
    let id = rb_num2uint(args[0]) as u32;
    let x = rb_num2int(args[1]) as i32;
    let y = rb_num2int(args[2]) as i32;
    let width = rb_num2int(args[3]).max(0) as i32;
    let height = rb_num2int(args[4]).max(0) as i32;
    let mut text_value = args[5];
    let text_ptr = unsafe { rb_string_value_cstr(&mut text_value) };
    if text_ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let text = unsafe { CStr::from_ptr(text_ptr) }
        .to_string_lossy()
        .to_string();
    let align = rb_num2int(args[6]) as i32;
    let font_size = rb_num2int(args[7]) as i32;
    let font_size = font_size.max(6);
    let color = unpack_color(rb_num2uint(args[8]) as u32);
    draw_text(
        id,
        RectData::new(x, y, width, height),
        &text,
        align,
        font_size,
        ColorData::new(
            color.0[0] as f32,
            color.0[1] as f32,
            color.0[2] as f32,
            color.0[3] as f32,
        ),
    );
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_text_size(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let font_size = rb_num2int(*argv) as i32;
    let font_size = font_size.max(6);
    let mut text_value = *argv.add(1);
    let text_ptr = rb_string_value_cstr(&mut text_value);
    if text_ptr.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let text = CStr::from_ptr(text_ptr).to_string_lossy().to_string();
    let (width, height) = text_size(font_size, &text);
    let array = rb_sys::rb_ary_new();
    rb_sys::rb_ary_push(array, int_to_value(width as i64));
    rb_sys::rb_ary_push(array, int_to_value(height as i64));
    array
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

fn gradient_fill_raw(
    image: &mut RgbaImage,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    start: Rgba<u8>,
    end: Rgba<u8>,
    vertical: bool,
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let width_u = image.width() as i32;
    let height_u = image.height() as i32;
    let steps = if vertical { height } else { width }.max(1);
    for offset in 0..steps {
        let t = offset as f32 / steps as f32;
        let mut color = [0u8; 4];
        for i in 0..4 {
            let s = start.0[i] as f32;
            let e = end.0[i] as f32;
            color[i] = (s + (e - s) * t).clamp(0.0, 255.0) as u8;
        }
        if vertical {
            let row = y + offset;
            if row < 0 || row >= height_u {
                continue;
            }
            for col in 0..width {
                let px = x + col;
                if px < 0 || px >= width_u {
                    continue;
                }
                let mut dst = image.get_pixel_mut(px as u32, row as u32).0;
                blend_rgba(&mut dst, color);
                *image.get_pixel_mut(px as u32, row as u32) = Rgba(dst);
            }
        } else {
            let col = x + offset;
            if col < 0 || col >= width_u {
                continue;
            }
            for row in 0..height {
                let py = y + row;
                if py < 0 || py >= height_u {
                    continue;
                }
                let mut dst = image.get_pixel_mut(col as u32, py as u32).0;
                blend_rgba(&mut dst, color);
                *image.get_pixel_mut(col as u32, py as u32) = Rgba(dst);
            }
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

fn color_to_rgba(color: ColorData) -> Rgba<u8> {
    let clamp = |v: f32| v.round().clamp(0.0, 255.0) as u8;
    Rgba([
        clamp(color.red),
        clamp(color.green),
        clamp(color.blue),
        clamp(color.alpha),
    ])
}

fn color_from_rgba(pixel: [u8; 4]) -> ColorData {
    ColorData::new(
        pixel[0] as f32,
        pixel[1] as f32,
        pixel[2] as f32,
        pixel[3] as f32,
    )
}

fn extract_region(
    image: &Arc<RgbaImage>,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Option<RgbaImage> {
    if width <= 0 || height <= 0 {
        return None;
    }
    let x = x.max(0) as u32;
    let y = y.max(0) as u32;
    let width = width as u32;
    let height = height as u32;
    if x >= image.width() || y >= image.height() {
        return None;
    }
    let max_width = (image.width() - x).min(width);
    let max_height = (image.height() - y).min(height);
    let mut region = RgbaImage::new(max_width, max_height);
    for yy in 0..max_height {
        for xx in 0..max_width {
            let pixel = image.get_pixel(x + xx, y + yy);
            region.put_pixel(xx, yy, *pixel);
        }
    }
    Some(region)
}

fn measure_text(text: &str, font_size: i32) -> (i32, i32) {
    let glyph_width = ((font_size as f32) * 0.6).max(1.0) as i32;
    let width = glyph_width * text.chars().count() as i32;
    let height = font_size.max(1);
    (width, height)
}

fn draw_text_run(
    image: &mut RgbaImage,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    text: &str,
    align: i32,
    font_size: i32,
    color: Rgba<u8>,
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let (text_width, glyph_height) = measure_text(text, font_size);
    let start_x = match align {
        1 => x + (width - text_width) / 2,
        2 => x + width - text_width,
        _ => x,
    };
    let start_y = y;
    let glyph_width = ((font_size as f32) * 0.6).max(1.0) as i32;
    for (index, ch) in text.chars().enumerate() {
        let gx = start_x + index as i32 * glyph_width;
        draw_char(image, gx, start_y, glyph_width, glyph_height, ch, color);
    }
}

fn draw_char(
    image: &mut RgbaImage,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    ch: char,
    color: Rgba<u8>,
) {
    if width <= 0 || height <= 0 {
        return;
    }
    let code = ch as usize;
    let glyph = if code < BASIC_LEGACY.len() {
        BASIC_LEGACY[code]
    } else {
        [0u8; 8]
    };
    let img_width = image.width() as i32;
    let img_height = image.height() as i32;
    for ty in 0..height {
        let src_y = ((ty * 8) / height).clamp(0, 7) as usize;
        let row = glyph[src_y];
        for tx in 0..width {
            let src_x = ((tx * 8) / width).clamp(0, 7);
            if (row >> src_x) & 1 == 1 {
                let px = x + tx;
                let py = y + ty;
                if px < 0 || py < 0 || px >= img_width || py >= img_height {
                    continue;
                }
                let mut dst = image.get_pixel_mut(px as u32, py as u32).0;
                blend_rgba(&mut dst, color.0);
                *image.get_pixel_mut(px as u32, py as u32) = Rgba(dst);
            }
        }
    }
}

fn normalize_hue(hue: i32) -> f32 {
    let mut value = hue % 360;
    if value < 0 {
        value += 360;
    }
    value as f32
}

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let mut h = 0.0;
    if delta > 0.0 {
        if (max - r).abs() < f32::EPSILON {
            h = 60.0 * ((g - b) / delta).rem_euclid(6.0);
        } else if (max - g).abs() < f32::EPSILON {
            h = 60.0 * (((b - r) / delta) + 2.0);
        } else {
            h = 60.0 * (((r - g) / delta) + 4.0);
        }
    }
    if h < 0.0 {
        h += 360.0;
    }

    let s = if max <= 0.0 { 0.0 } else { delta / max };
    let v = max;
    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    if s <= 0.0 {
        return (v, v, v);
    }
    let h_sector = h / 60.0;
    let i = h_sector.floor();
    let f = h_sector - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match (i as i32) % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}
unsafe extern "C" fn bitmap_hue_change(argc: c_int, argv: *const VALUE, _self: VALUE) -> VALUE {
    if argc != 2 || argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = std::slice::from_raw_parts(argv, 2);
    let id = rb_num2uint(args[0]) as u32;
    let hue = rb_num2int(args[1]) as i32;
    hue_change(id, hue);
    rb_sys::Qnil as VALUE
}
