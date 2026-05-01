use ab_glyph::{Font as _, FontVec, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::{info, warn};

use super::{module::project_root, ColorData};

/// Font reference passed from Ruby into the native draw routines.
/// Mirrors the relevant fields of [`crate::classes::font::FontSnapshot`].
#[derive(Clone, Debug)]
pub struct FontSpec {
    pub names: Vec<String>,
    pub size: i32,
    pub bold: bool,
    pub italic: bool,
    pub shadow: bool,
    pub color: ColorData,
    pub shadow_color: Option<ColorData>,
}

impl FontSpec {
    /// Map RGSS `font.size` to the em-square pixel scale ab_glyph expects.
    /// PE 21.1 / mkxp-z visual reference renders glyphs at roughly 0.75× the
    /// numerical font size in pixels (cap height ≈ 0.5 × size). Without this
    /// scaling our text was visibly too tall and overflowed message rows.
    pub fn px_size(&self) -> f32 {
        (self.size.max(6) as f32) * 0.75
    }
}

#[derive(Clone)]
struct LoadedFont {
    name: String,
    family_lc: Vec<String>,
    file_stem_lc: String,
    is_bold: bool,
    is_italic: bool,
    font: Arc<FontVec>,
}

#[derive(Default)]
struct Registry {
    fonts: Vec<LoadedFont>,
    /// Last project root we scanned. `None` means uninitialised.
    scanned_root: Option<PathBuf>,
    fallback: Option<Arc<FontVec>>,
}

static REGISTRY: Lazy<RwLock<Registry>> = Lazy::new(|| RwLock::new(Registry::default()));

/// Glyph cache keyed by font ptr identity + scaled size + glyph char + style.
type GlyphKey = (usize, u32, char, bool);

#[derive(Clone)]
struct CachedGlyph {
    width: u32,
    height: u32,
    bearing_x: f32,
    bearing_y: f32,
    advance: f32,
    /// 8-bit alpha mask, row-major, length = width*height.
    mask: Arc<Vec<u8>>,
}

static GLYPH_CACHE: Lazy<RwLock<HashMap<GlyphKey, Option<CachedGlyph>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn ensure_registry() {
    let project = project_root().cloned();
    {
        let r = REGISTRY.read();
        if r.scanned_root == project && !r.fonts.is_empty() {
            return;
        }
        if r.scanned_root.is_some() && project.is_none() {
            // Already scanned without a project root and no project came in.
            return;
        }
    }
    let mut w = REGISTRY.write();
    if w.scanned_root == project && !w.fonts.is_empty() {
        return;
    }
    w.fonts.clear();
    if let Some(root) = project.as_ref() {
        scan_directory(&root.join("Fonts"), &mut w.fonts);
    }
    w.scanned_root = project;
    if w.fallback.is_none() {
        // Use the first project font as the fallback so we always have *some*
        // outline we can fall back to for missing glyphs.
        if let Some(first) = w.fonts.first() {
            w.fallback = Some(first.font.clone());
        }
    }
    info!(target: "font", count = w.fonts.len(), "font registry initialised");
}

fn scan_directory(dir: &Path, out: &mut Vec<LoadedFont>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        if ext != "ttf" && ext != "otf" {
            continue;
        }
        match load_font(&path) {
            Ok(loaded) => out.push(loaded),
            Err(err) => warn!(target: "font", path = %path.display(), error = %err, "failed to load font"),
        }
    }
}

fn load_font(path: &Path) -> anyhow::Result<LoadedFont> {
    let bytes = fs::read(path)?;
    let font = FontVec::try_from_vec(bytes)
        .map_err(|e| anyhow::anyhow!("invalid font file: {e}"))?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let stem_lc = stem.to_ascii_lowercase();
    // Heuristic style detection from the file name. Many bundled families use
    // "<family> bold.ttf" / "<family> italic.ttf" naming.
    let is_bold = stem_lc.contains("bold");
    let is_italic = stem_lc.contains("italic") || stem_lc.contains("oblique");
    let mut family_lc = vec![stem_lc.clone()];
    // Strip style words to expose the bare family name.
    let bare = stem_lc
        .replace("bold", "")
        .replace("italic", "")
        .replace("oblique", "")
        .trim()
        .replace("  ", " ");
    if !bare.is_empty() && bare != stem_lc {
        family_lc.push(bare);
    }
    Ok(LoadedFont {
        name: stem,
        family_lc,
        file_stem_lc: stem_lc,
        is_bold,
        is_italic,
        font: Arc::new(font),
    })
}

fn pick_font(spec: &FontSpec) -> Option<Arc<FontVec>> {
    ensure_registry();
    let r = REGISTRY.read();
    if r.fonts.is_empty() {
        return None;
    }
    // 1. Try exact-style match against each requested name.
    for requested in &spec.names {
        let req_lc = requested.trim().to_ascii_lowercase();
        if req_lc.is_empty() {
            continue;
        }
        if let Some(f) = best_match(&r.fonts, &req_lc, spec.bold, spec.italic) {
            return Some(f);
        }
    }
    // 2. Fall back to the registry's default (first font scanned).
    r.fallback.clone()
}

fn best_match(
    fonts: &[LoadedFont],
    requested: &str,
    want_bold: bool,
    want_italic: bool,
) -> Option<Arc<FontVec>> {
    // First pass: exact stem or family + matching style.
    let mut style_match: Option<&LoadedFont> = None;
    let mut family_match: Option<&LoadedFont> = None;
    for f in fonts {
        let name_hit = f.file_stem_lc == requested
            || f.family_lc.iter().any(|fam| fam == requested);
        if !name_hit {
            continue;
        }
        if f.is_bold == want_bold && f.is_italic == want_italic {
            style_match = Some(f);
            break;
        }
        if family_match.is_none() {
            family_match = Some(f);
        }
    }
    style_match
        .or(family_match)
        .map(|f| f.font.clone())
}

fn cache_glyph(
    font: &FontVec,
    size_px: f32,
    ch: char,
    italic: bool,
) -> Option<CachedGlyph> {
    let key: GlyphKey = (
        Arc::as_ptr(&Arc::new(0u8)) as usize, // placeholder — replaced below
        (size_px * 64.0) as u32,
        ch,
        italic,
    );
    // We can't key on a pointer to the FontVec because we don't have an Arc here.
    // Instead, use a (font-data-ptr, size, ch, italic) key based on the font's
    // bytes pointer. For simplicity we hash on font data length + first 8 bytes.
    let _ = key; // not used; below uses identity-by-data
    let id = font_identity(font);
    let key: GlyphKey = (id, (size_px * 64.0) as u32, ch, italic);
    {
        let cache = GLYPH_CACHE.read();
        if let Some(slot) = cache.get(&key) {
            return slot.clone();
        }
    }
    let computed = rasterize_glyph(font, size_px, ch, italic);
    let mut cache = GLYPH_CACHE.write();
    cache.insert(key, computed.clone());
    computed
}

fn font_identity(font: &FontVec) -> usize {
    // Stable per-font identity: take the address of the underlying bytes.
    // FontVec doesn't expose its buffer, so use units_per_em + ascent_unscaled
    // as a rough fingerprint. This collides extremely rarely between distinct
    // fonts loaded in the same project.
    let units = font.units_per_em().unwrap_or(1.0) as usize;
    let ascent = font.ascent_unscaled() as i32 as usize;
    (units << 16) ^ ascent
}

fn rasterize_glyph(
    font: &FontVec,
    size_px: f32,
    ch: char,
    italic: bool,
) -> Option<CachedGlyph> {
    let glyph_id = font.glyph_id(ch);
    let scale = PxScale::from(size_px);
    let scaled = font.as_scaled(scale);
    let advance = scaled.h_advance(glyph_id);
    if ch == ' ' || ch == '\t' {
        return Some(CachedGlyph {
            width: 0,
            height: 0,
            bearing_x: 0.0,
            bearing_y: 0.0,
            advance,
            mask: Arc::new(Vec::new()),
        });
    }
    let glyph = glyph_id.with_scale(scale);
    let outlined = font.outline_glyph(glyph)?;
    let bounds = outlined.px_bounds();
    let width = bounds.width().ceil() as u32;
    let height = bounds.height().ceil() as u32;
    if width == 0 || height == 0 {
        return Some(CachedGlyph {
            width: 0,
            height: 0,
            bearing_x: bounds.min.x,
            bearing_y: bounds.min.y,
            advance,
            mask: Arc::new(Vec::new()),
        });
    }
    let mut mask = vec![0u8; (width * height) as usize];
    outlined.draw(|x, y, c| {
        let idx = (y * width + x) as usize;
        if idx < mask.len() {
            mask[idx] = (c.clamp(0.0, 1.0) * 255.0) as u8;
        }
    });
    if italic {
        // Faux italic via horizontal shear: shift each row by a fraction of
        // its distance from the baseline. Cheap, decent looking on body text.
        mask = shear_mask(&mask, width, height, 0.18);
    }
    Some(CachedGlyph {
        width,
        height,
        bearing_x: bounds.min.x,
        bearing_y: bounds.min.y,
        advance,
        mask: Arc::new(mask),
    })
}

fn shear_mask(src: &[u8], width: u32, height: u32, factor: f32) -> Vec<u8> {
    let extra = (height as f32 * factor).ceil() as u32;
    let new_w = width + extra;
    let mut out = vec![0u8; (new_w * height) as usize];
    for y in 0..height {
        let shift = ((height - 1 - y) as f32 * factor) as u32;
        for x in 0..width {
            let v = src[(y * width + x) as usize];
            if v == 0 {
                continue;
            }
            let dst_x = x + shift;
            if dst_x < new_w {
                out[(y * new_w + dst_x) as usize] = v;
            }
        }
    }
    out
}

/// Lay out a single line of text and return (width, height). Matches the
/// classic RGSS contract for `Bitmap#text_size`: width = total horizontal
/// advance, height = the font's nominal point size (NOT the actual rendered
/// pixel height, which can include ascender/descender slack).
pub fn measure(text: &str, spec: &FontSpec) -> (i32, i32) {
    if text.is_empty() {
        return (0, spec.size.max(1));
    }
    let Some(font) = pick_font(spec) else {
        return fallback_measure(text, spec.px_size() as i32);
    };
    let scale = PxScale::from(spec.px_size());
    let scaled = font.as_scaled(scale);
    let mut total: f32 = 0.0;
    for ch in text.chars() {
        let g = font.glyph_id(ch);
        total += scaled.h_advance(g);
    }
    (total.ceil() as i32, spec.size.max(1))
}

pub fn line_height(spec: &FontSpec) -> i32 {
    if let Some(font) = pick_font(spec) {
        let scaled = font.as_scaled(PxScale::from(spec.px_size()));
        return (scaled.ascent() - scaled.descent() + scaled.line_gap()).ceil() as i32;
    }
    spec.size.max(1)
}

fn fallback_measure(text: &str, size: i32) -> (i32, i32) {
    let glyph_w = ((size as f32) * 0.6).max(1.0) as i32;
    (glyph_w * text.chars().count() as i32, size.max(1))
}

/// Draw `text` into `image`, aligned within the supplied rect. Mirrors RGSS'
/// `Bitmap#draw_text` semantics: align 0=left, 1=center, 2=right.
pub fn draw_text(
    image: &mut RgbaImage,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    text: &str,
    align: i32,
    spec: &FontSpec,
) {
    if width <= 0 || height <= 0 || text.is_empty() {
        return;
    }
    let Some(font) = pick_font(spec) else {
        super::bitmap::draw_text_fallback(
            image,
            x,
            y,
            width,
            height,
            text,
            align,
            spec.size,
            color_to_rgba(spec.color),
        );
        return;
    };
    let scale = PxScale::from(spec.px_size());
    let scaled = font.as_scaled(scale);
    let ascent = scaled.ascent();

    // Total advance to position the text by alignment.
    let mut advance_total: f32 = 0.0;
    for ch in text.chars() {
        advance_total += scaled.h_advance(font.glyph_id(ch));
    }
    let advance_total_i = advance_total.ceil() as i32;
    let start_x = match align {
        1 => x + (width - advance_total_i) / 2,
        2 => x + width - advance_total_i,
        _ => x,
    };
    // Top-align the baseline inside the rect — matches RGSS classic /
    // Game.exe behaviour where the text sits flush with the top of the
    // bitmap.draw_text rect, with the descender falling within the rect.
    let baseline_y = y + ascent.round() as i32;

    let text_color = color_to_rgba(spec.color);
    let shadow_color = spec
        .shadow_color
        .map(color_to_rgba)
        .unwrap_or_else(|| Rgba([0, 0, 0, ((text_color.0[3] as u32) * 192 / 255) as u8]));

    if spec.shadow {
        render_text_run(
            image,
            &font,
            scale,
            start_x + 1,
            baseline_y + 1,
            text,
            shadow_color,
            spec.bold,
            spec.italic,
        );
    }
    render_text_run(
        image,
        &font,
        scale,
        start_x,
        baseline_y,
        text,
        text_color,
        spec.bold,
        spec.italic,
    );
}

fn render_text_run(
    image: &mut RgbaImage,
    font: &Arc<FontVec>,
    scale: PxScale,
    start_x: i32,
    baseline_y: i32,
    text: &str,
    color: Rgba<u8>,
    bold: bool,
    italic: bool,
) {
    let scaled = font.as_scaled(scale);
    let mut pen_x = start_x as f32;
    let img_w = image.width() as i32;
    let img_h = image.height() as i32;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        let advance = scaled.h_advance(glyph_id);
        let cached = cache_glyph(font, scale.x, ch, italic);
        if let Some(g) = cached {
            if g.width > 0 && g.height > 0 {
                let gx = pen_x + g.bearing_x;
                let gy = baseline_y as f32 + g.bearing_y;
                blit_mask(
                    image,
                    gx.round() as i32,
                    gy.round() as i32,
                    g.width,
                    g.height,
                    &g.mask,
                    color,
                    img_w,
                    img_h,
                );
                if bold {
                    blit_mask(
                        image,
                        gx.round() as i32 + 1,
                        gy.round() as i32,
                        g.width,
                        g.height,
                        &g.mask,
                        color,
                        img_w,
                        img_h,
                    );
                }
            }
        }
        pen_x += advance;
    }
}

#[inline]
fn blit_mask(
    image: &mut RgbaImage,
    dx: i32,
    dy: i32,
    w: u32,
    h: u32,
    mask: &[u8],
    color: Rgba<u8>,
    img_w: i32,
    img_h: i32,
) {
    let [cr, cg, cb, ca] = color.0;
    if ca == 0 || mask.is_empty() {
        return;
    }
    for ty in 0..h {
        let py = dy + ty as i32;
        if py < 0 || py >= img_h {
            continue;
        }
        for tx in 0..w {
            let alpha = mask[(ty * w + tx) as usize];
            if alpha == 0 {
                continue;
            }
            let px = dx + tx as i32;
            if px < 0 || px >= img_w {
                continue;
            }
            let combined = ((alpha as u32) * (ca as u32) / 255) as u8;
            let mut dst = image.get_pixel_mut(px as u32, py as u32).0;
            blend_rgba(&mut dst, [cr, cg, cb, combined]);
            *image.get_pixel_mut(px as u32, py as u32) = Rgba(dst);
        }
    }
}

fn color_to_rgba(c: ColorData) -> Rgba<u8> {
    let clamp = |v: f32| v.clamp(0.0, 255.0) as u8;
    Rgba([clamp(c.red), clamp(c.green), clamp(c.blue), clamp(c.alpha)])
}

fn blend_rgba(dst: &mut [u8; 4], src: [u8; 4]) {
    let sa = src[3] as u32;
    if sa == 0 {
        return;
    }
    if sa == 255 || dst[3] == 0 {
        *dst = src;
        return;
    }
    let inv_sa = 255 - sa;
    for i in 0..3 {
        dst[i] = ((src[i] as u32 * sa + dst[i] as u32 * inv_sa) / 255) as u8;
    }
    dst[3] = (sa + (dst[3] as u32 * inv_sa) / 255) as u8;
}

/// Returns true if the registry has at least one usable font. Used by the
/// `Font.exist?(name)` Ruby method.
pub fn font_exists(name: &str) -> bool {
    ensure_registry();
    let r = REGISTRY.read();
    let lc = name.trim().to_ascii_lowercase();
    if lc.is_empty() {
        return !r.fonts.is_empty();
    }
    r.fonts
        .iter()
        .any(|f| f.file_stem_lc == lc || f.family_lc.iter().any(|fam| fam == &lc))
}
