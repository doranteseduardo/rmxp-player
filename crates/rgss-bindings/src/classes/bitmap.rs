use super::{
    color::{get_color_data, new_color},
    common::{
        bool_to_value, define_method, define_singleton_method, get_typed_data, install_allocator,
        int_to_value, wrap_typed_data, DataTypeBuilder, StaticDataType,
    },
    font::{self, clone_font, font_snapshot, is_font, new_font, FontSnapshot},
    rect,
};
use crate::native::{self, value_to_bool, value_to_i32, ColorData, RectData};
use anyhow::Result;
use once_cell::sync::OnceCell;
use rb_sys::{
    bindings::{rb_cString, rb_gc_mark, rb_obj_as_string, rb_obj_is_kind_of, rb_string_value_cstr},
    VALUE,
};
use std::{
    ffi::{c_void, CStr},
    os::raw::c_int,
    ptr, slice,
    sync::atomic::{AtomicU32, Ordering},
};
use tracing::warn;

const BITMAP_CLASS_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"Bitmap\0") };
const BITMAP_STRUCT_NAME: &CStr = unsafe { CStr::from_bytes_with_nul_unchecked(b"RGSS::Bitmap\0") };

static BITMAP_TYPE: StaticDataType = StaticDataType::new(|| {
    DataTypeBuilder::new(BITMAP_STRUCT_NAME)
        .mark(bitmap_mark)
        .free(bitmap_free)
});
static BITMAP_CLASS: OnceCell<VALUE> = OnceCell::new();
static MAX_SIZE: AtomicU32 = AtomicU32::new(16_384);

#[derive(Clone)]
struct BitmapValue {
    handle: u32,
    disposed: bool,
    font: VALUE,
}

impl Default for BitmapValue {
    fn default() -> Self {
        Self {
            handle: 0,
            disposed: true,
            font: rb_sys::Qnil as VALUE,
        }
    }
}

pub fn init() -> Result<()> {
    unsafe {
        let klass = super::common::define_ruby_class(BITMAP_CLASS_NAME, None);
        let _ = BITMAP_CLASS.set(klass);
        install_allocator(klass, Some(bitmap_allocate));
        define_method(klass, cstr(b"initialize\0"), bitmap_initialize, -1);
        define_method(klass, cstr(b"dispose\0"), bitmap_dispose, 0);
        define_method(klass, cstr(b"disposed?\0"), bitmap_disposed_q, 0);
        define_method(klass, cstr(b"width\0"), bitmap_width, 0);
        define_method(klass, cstr(b"height\0"), bitmap_height, 0);
        define_method(klass, cstr(b"rect\0"), bitmap_rect, 0);
        define_method(klass, cstr(b"hue_change\0"), bitmap_hue_change, -1);
        define_method(klass, cstr(b"clear\0"), bitmap_clear, 0);
        define_method(klass, cstr(b"fill_rect\0"), bitmap_fill_rect, -1);
        define_method(
            klass,
            cstr(b"gradient_fill_rect\0"),
            bitmap_gradient_fill_rect,
            -1,
        );
        define_method(klass, cstr(b"blt\0"), bitmap_blt, -1);
        define_method(klass, cstr(b"stretch_blt\0"), bitmap_stretch_blt, -1);
        define_method(klass, cstr(b"get_pixel\0"), bitmap_get_pixel, -1);
        define_method(klass, cstr(b"set_pixel\0"), bitmap_set_pixel, -1);
        define_method(klass, cstr(b"text_size\0"), bitmap_text_size, -1);
        define_method(klass, cstr(b"draw_text\0"), bitmap_draw_text, -1);
        define_method(klass, cstr(b"dup\0"), bitmap_dup, 0);
        define_method(klass, cstr(b"clone\0"), bitmap_dup, 0);
        define_method(klass, cstr(b"font\0"), bitmap_get_font, 0);
        define_method(klass, cstr(b"font=\0"), bitmap_set_font, -1);

        define_singleton_method(klass, cstr(b"max_size\0"), bitmap_max_size, 0);
        define_singleton_method(klass, cstr(b"max_size=\0"), bitmap_set_max_size, -1);
        define_singleton_method(klass, cstr(b"_native_wrap\0"), bitmap_native_wrap, -1);
    }
    Ok(())
}

unsafe extern "C" fn bitmap_allocate(klass: VALUE) -> VALUE {
    bitmap_allocate_internal(klass)
}

unsafe fn bitmap_allocate_internal(klass: VALUE) -> VALUE {
    wrap_typed_data(klass, BitmapValue::default(), BITMAP_TYPE.as_rb_type())
}

fn bitmap_class() -> VALUE {
    *BITMAP_CLASS.get().expect("Bitmap not initialised")
}

unsafe extern "C" fn bitmap_mark(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = &*(ptr as *mut BitmapValue);
    if value.font != rb_sys::Qnil as VALUE {
        rb_gc_mark(value.font);
    }
}

unsafe extern "C" fn bitmap_free(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let value = Box::<BitmapValue>::from_raw(ptr as *mut BitmapValue);
    if !value.disposed && value.handle != 0 {
        native::bitmap::dispose(value.handle);
    }
}

fn get_bitmap(value: VALUE) -> &'static mut BitmapValue {
    unsafe { get_typed_data(value, BITMAP_TYPE.as_rb_type()) }.expect("Bitmap missing native data")
}

fn ensure_font_value(data: &mut BitmapValue) {
    if data.font == rb_sys::Qnil as VALUE {
        data.font = new_font();
    }
}

fn font_info(data: &BitmapValue) -> FontSnapshot {
    font_snapshot(data.font).unwrap_or(FontSnapshot {
        names: vec!["Arial".to_string()],
        size: 24,
        bold: false,
        italic: false,
        shadow: false,
        color: ColorData::new(255.0, 255.0, 255.0, 255.0),
    })
}

unsafe extern "C" fn bitmap_initialize(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = if argc <= 0 || argv.is_null() {
        &[]
    } else {
        slice::from_raw_parts(argv, argc as usize)
    };
    let data = get_bitmap(self_value);
    if data.handle != 0 && !data.disposed {
        native::bitmap::dispose(data.handle);
    }
    let mut handle = 0;
    if let Some(first) = args.first() {
        if rb_obj_is_kind_of(*first, rb_cString) != 0 {
            match load_from_string(*first) {
                Some(id) => {
                    handle = id;
                }
                None => {
                    warn!(target: "rgss", "Bitmap load failed, allocating blank");
                }
            }
        } else {
            let width = clamp_size(value_to_i32(*first));
            let height = if args.len() >= 2 {
                clamp_size(value_to_i32(args[1]))
            } else {
                width
            };
            handle = native::bitmap::create_blank(width, height);
        }
    }
    if handle == 0 {
        handle = native::bitmap::create_blank(32, 32);
    }
    data.handle = handle;
    data.disposed = false;
    data.font = new_font();
    self_value
}

unsafe extern "C" fn bitmap_dispose(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let data = get_bitmap(self_value);
    if !data.disposed && data.handle != 0 {
        native::bitmap::dispose(data.handle);
        data.disposed = true;
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_disposed_q(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let data = get_bitmap(self_value);
    bool_to_value(data.disposed || native::bitmap::is_disposed(data.handle))
}

unsafe extern "C" fn bitmap_width(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let data = get_bitmap(self_value);
    let width = native::bitmap::dimensions(data.handle)
        .map(|(w, _)| w)
        .unwrap_or(0);
    int_to_value(width as i64)
}

unsafe extern "C" fn bitmap_height(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let data = get_bitmap(self_value);
    let height = native::bitmap::dimensions(data.handle)
        .map(|(_, h)| h)
        .unwrap_or(0);
    int_to_value(height as i64)
}

unsafe extern "C" fn bitmap_rect(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let width_value = bitmap_width(0, ptr::null(), self_value);
    let height_value = bitmap_height(0, ptr::null(), self_value);
    let w = value_to_i32(width_value);
    let h = value_to_i32(height_value);
    rect::new_rect(0, 0, w, h)
}

unsafe extern "C" fn bitmap_hue_change(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let hue = value_to_i32(*argv);
    let data = get_bitmap(self_value);
    if data.disposed || data.handle == 0 {
        return rb_sys::Qnil as VALUE;
    }
    native::bitmap::hue_change(data.handle, hue);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_clear(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let data = get_bitmap(self_value);
    native::bitmap::clear(data.handle);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_fill_rect(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = slice_from(argc, argv);
    if let Some((rect, color)) = normalize_rect_color(&args) {
        let color = ensure_color(color);
        native::bitmap::fill_rect(get_bitmap(self_value).handle, rect, color);
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_gradient_fill_rect(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = slice_from(argc, argv);
    if let Some((rect, c1, c2, vertical)) = normalize_gradient_args(&args) {
        let color1 = ensure_color(c1);
        let color2 = ensure_color(c2);
        native::bitmap::gradient_fill_rect(
            get_bitmap(self_value).handle,
            rect,
            color1,
            color2,
            vertical,
        );
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_blt(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = slice_from(argc, argv);
    if args.len() < 4 {
        return rb_sys::Qnil as VALUE;
    }
    let dest_x = value_to_i32(args[0]);
    let dest_y = value_to_i32(args[1]);
    let src_bitmap = args[2];
    let src_rect_value = args.get(3).copied();
    let opacity = args.get(4).map(|v| value_to_i32(*v)).unwrap_or(255);
    if let Some(src) = get_typed_data::<BitmapValue>(src_bitmap, BITMAP_TYPE.as_rb_type()) {
        let rect = src_rect_value
            .and_then(rect::rect_data)
            .unwrap_or_else(|| full_rect(src.handle));
        native::bitmap::blt(
            get_bitmap(self_value).handle,
            dest_x,
            dest_y,
            src.handle,
            rect,
            opacity.clamp(0, 255) as u8,
        );
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_stretch_blt(
    argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let args = slice_from(argc, argv);
    if args.len() < 5 {
        return rb_sys::Qnil as VALUE;
    }
    let (dest_rect, src_bitmap, src_rect_value, opacity) = normalize_stretch_args(&args);
    if let Some(src_value) = src_bitmap {
        if let Some(src) = get_typed_data::<BitmapValue>(src_value, BITMAP_TYPE.as_rb_type()) {
            let src_rect = src_rect_value.unwrap_or_else(|| full_rect(src.handle));
            native::bitmap::stretch_blt(
                get_bitmap(self_value).handle,
                dest_rect,
                src.handle,
                src_rect,
                opacity,
            );
        }
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_get_pixel(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let args = slice::from_raw_parts(argv, 2);
    let x = value_to_i32(args[0]);
    let y = value_to_i32(args[1]);
    if let Some(color) = native::bitmap::get_pixel(get_bitmap(self_value).handle, x, y) {
        return new_color(color.red, color.green, color.blue, color.alpha);
    }
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_set_pixel(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = slice_from(argc, argv);
    if args.len() < 3 {
        return rb_sys::Qnil as VALUE;
    }
    let x = value_to_i32(args[0]);
    let y = value_to_i32(args[1]);
    let color = ensure_color(args[2]);
    native::bitmap::set_pixel(get_bitmap(self_value).handle, x, y, color);
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_text_size(
    _argc: c_int,
    argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    if argv.is_null() {
        return rect::new_rect(0, 0, 0, 0);
    }
    let text = value_to_string(*argv);
    let font_info = font_info(get_bitmap(self_value));
    let (width, height) = native::bitmap::text_size(font_info.size, &text);
    rect::new_rect(0, 0, width, height)
}

unsafe extern "C" fn bitmap_draw_text(argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    let args = slice_from(argc, argv);
    if args.len() < 2 {
        return rb_sys::Qnil as VALUE;
    }
    let (rect, text, align) = normalize_draw_text_args(&args);
    let info = font_info(get_bitmap(self_value));
    native::bitmap::draw_text(
        get_bitmap(self_value).handle,
        rect,
        &text,
        align,
        info.size,
        info.color,
    );
    rb_sys::Qnil as VALUE
}

unsafe extern "C" fn bitmap_dup(_argc: c_int, _argv: *const VALUE, self_value: VALUE) -> VALUE {
    let data = get_bitmap(self_value);
    let new_bitmap = new_instance();
    if let Some(new_handle) = native::bitmap::copy_bitmap(data.handle) {
        let target = get_bitmap(new_bitmap);
        target.handle = new_handle;
        target.disposed = false;
        target.font = clone_font(data.font);
    }
    new_bitmap
}

unsafe extern "C" fn bitmap_get_font(
    _argc: c_int,
    _argv: *const VALUE,
    self_value: VALUE,
) -> VALUE {
    let bitmap = get_bitmap(self_value);
    ensure_font_value(bitmap);
    bitmap.font
}

unsafe extern "C" fn bitmap_set_font(_argc: c_int, argv: *const VALUE, self_value: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let value = *argv;
    let bitmap = get_bitmap(self_value);
    bitmap.font = if is_font(value) {
        font::clone_font(value)
    } else {
        new_font()
    };
    value
}

unsafe extern "C" fn bitmap_max_size(_argc: c_int, _argv: *const VALUE, _klass: VALUE) -> VALUE {
    int_to_value(MAX_SIZE.load(Ordering::Relaxed) as i64)
}

unsafe extern "C" fn bitmap_set_max_size(_argc: c_int, argv: *const VALUE, _klass: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let size = value_to_i32(*argv).max(1) as u32;
    MAX_SIZE.store(size, Ordering::Relaxed);
    *argv
}

unsafe extern "C" fn bitmap_native_wrap(_argc: c_int, argv: *const VALUE, klass: VALUE) -> VALUE {
    if argv.is_null() {
        return rb_sys::Qnil as VALUE;
    }
    let handle = rb_sys::rb_num2uint(*argv) as u32;
    unsafe {
        let value = bitmap_allocate_internal(klass);
        let data = get_bitmap(value);
        data.handle = handle;
        data.disposed = false;
        data.font = new_font();
        value
    }
}

fn full_rect(handle: u32) -> RectData {
    native::bitmap::dimensions(handle)
        .map(|(w, h)| RectData::new(0, 0, w as i32, h as i32))
        .unwrap_or_else(|| RectData::new(0, 0, 0, 0))
}

fn slice_from<'a>(argc: c_int, argv: *const VALUE) -> &'a [VALUE] {
    if argc <= 0 || argv.is_null() {
        &[]
    } else {
        unsafe { slice::from_raw_parts(argv, argc as usize) }
    }
}

fn clamp_size(value: i32) -> u32 {
    let max = MAX_SIZE.load(Ordering::Relaxed);
    value.max(1).min(max as i32) as u32
}

fn ensure_color(value: VALUE) -> ColorData {
    let color = get_color_data(value);
    ColorData::new(color.red, color.green, color.blue, color.alpha)
}

fn normalize_rect_color(args: &[VALUE]) -> Option<(RectData, VALUE)> {
    if args.len() == 2 {
        if let Some(rect) = rect::rect_data(args[0]) {
            return Some((rect, args[1]));
        }
    } else if args.len() >= 5 {
        let rect = RectData::new(
            value_to_i32(args[0]),
            value_to_i32(args[1]),
            value_to_i32(args[2]),
            value_to_i32(args[3]),
        );
        return Some((rect, args[4]));
    }
    None
}

fn normalize_gradient_args(args: &[VALUE]) -> Option<(RectData, VALUE, VALUE, bool)> {
    if args.len() >= 4 && rect::rect_data(args[0]).is_some() {
        let rect = rect::rect_data(args[0])?;
        let vertical = args.get(3).map(|v| value_to_bool(*v)).unwrap_or(false);
        return Some((rect, args[1], args[2], vertical));
    } else if args.len() >= 7 {
        let rect = RectData::new(
            value_to_i32(args[0]),
            value_to_i32(args[1]),
            value_to_i32(args[2]),
            value_to_i32(args[3]),
        );
        let vertical = args.get(6).map(|v| value_to_bool(*v)).unwrap_or(false);
        return Some((rect, args[4], args[5], vertical));
    }
    None
}

fn normalize_stretch_args(args: &[VALUE]) -> (RectData, Option<VALUE>, Option<RectData>, u8) {
    if let Some(rect_value) = args.first() {
        if let Some(rect) = rect::rect_data(*rect_value) {
            let src_bitmap = args.get(1).copied();
            let src_rect = args.get(2).and_then(|v| rect::rect_data(*v));
            let opacity = args
                .get(3)
                .map(|v| value_to_i32(*v).clamp(0, 255) as u8)
                .unwrap_or(255);
            return (rect, src_bitmap, src_rect, opacity);
        }
    }
    if args.len() >= 10 {
        let rect = RectData::new(
            int_arg(args, 0),
            int_arg(args, 1),
            int_arg(args, 2),
            int_arg(args, 3),
        );
        let src_bitmap = args.get(4).copied();
        let src_rect = Some(RectData::new(
            int_arg(args, 5),
            int_arg(args, 6),
            int_arg(args, 7),
            int_arg(args, 8),
        ));
        let opacity = args
            .get(9)
            .map(|v| value_to_i32(*v).clamp(0, 255) as u8)
            .unwrap_or(255);
        (rect, src_bitmap, src_rect, opacity)
    } else {
        (RectData::new(0, 0, 0, 0), None, None, 255)
    }
}

fn normalize_draw_text_args(args: &[VALUE]) -> (RectData, String, i32) {
    if !args.is_empty() {
        if let Some(rect) = rect::rect_data(args[0]) {
            let text = if args.len() >= 2 {
                value_to_string(args[1])
            } else {
                String::new()
            };
            let align = int_arg(args, 2);
            return (rect, text, align);
        }
    }
    let rect = RectData::new(
        int_arg(args, 0),
        int_arg(args, 1),
        int_arg(args, 2),
        int_arg(args, 3),
    );
    let text = args.get(4).map(|v| value_to_string(*v)).unwrap_or_default();
    let align = int_arg(args, 5);
    (rect, text, align)
}

fn int_arg(args: &[VALUE], index: usize) -> i32 {
    args.get(index).map(|v| value_to_i32(*v)).unwrap_or(0)
}

fn value_to_string(value: VALUE) -> String {
    unsafe {
        let mut coerced = rb_obj_as_string(value);
        let ptr = rb_string_value_cstr(&mut coerced);
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn load_from_string(value: VALUE) -> Option<u32> {
    unsafe {
        let mut coerced = rb_obj_as_string(value);
        let ptr = rb_string_value_cstr(&mut coerced);
        if ptr.is_null() {
            return None;
        }
        let path = CStr::from_ptr(ptr).to_string_lossy().to_string();
        match native::bitmap::load_relative(&path) {
            Ok(handle) => Some(handle),
            Err(err) => {
                warn!(target: "rgss", %path, %err, "Bitmap load failed");
                None
            }
        }
    }
}

fn new_instance() -> VALUE {
    unsafe { bitmap_allocate_internal(bitmap_class()) }
}

const fn cstr(bytes: &'static [u8]) -> &'static CStr {
    unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
}

pub fn is_bitmap(value: VALUE) -> bool {
    unsafe { get_typed_data::<BitmapValue>(value, BITMAP_TYPE.as_rb_type()).is_some() }
}

pub fn bitmap_handle(value: VALUE) -> Option<u32> {
    unsafe { get_typed_data::<BitmapValue>(value, BITMAP_TYPE.as_rb_type()) }.and_then(|bitmap| {
        if bitmap.disposed {
            None
        } else {
            Some(bitmap.handle)
        }
    })
}
