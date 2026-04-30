use image::RgbaImage;
use render::{
    AutotileTexture, Camera, ClipRect, FallbackScene, PlaneInstance, RenderFrame, SpriteInstance,
    TileScene, TilemapFlashMap, TilemapInstance, WindowInstance,
};
use rgss_bindings::{
    bitmap_snapshot, plane_snapshot, sprite_snapshot, tilemap_snapshot, viewport_snapshot,
    window_snapshot, TilemapData,
};
use std::collections::HashMap;
use std::sync::Arc;

const WINDOW_PADDING: i32 = 16;

pub struct GameState {
    fallback_scene: Option<TileScene>,
    fallback_camera: Option<Camera>,
    screen_size: (u32, u32),
    tilemaps: Vec<TilemapInstance>,
    sprites: Vec<SpriteInstance>,
    planes: Vec<PlaneInstance>,
    windows: Vec<WindowInstance>,
}

impl GameState {
    pub fn new(scene: Option<TileScene>, start: (f32, f32), viewport: (u32, u32)) -> Self {
        let fallback_camera = scene
            .as_ref()
            .map(|scene| camera_centered(scene, start, viewport));
        Self {
            fallback_scene: scene,
            fallback_camera,
            screen_size: viewport,
            tilemaps: Vec::new(),
            sprites: Vec::new(),
            planes: Vec::new(),
            windows: Vec::new(),
        }
    }

    pub fn render_frame(&mut self) -> Option<RenderFrame<'_>> {
        let textures = self.collect_textures();
        self.rebuild_rgss_state(&textures);
        if self.has_rgss_frame() {
            return Some(RenderFrame {
                tilemaps: &self.tilemaps,
                planes: &self.planes,
                sprites: &self.sprites,
                windows: &self.windows,
                fallback: None,
            });
        }
        let scene = self.fallback_scene.as_ref()?;
        let fallback = FallbackScene {
            scene,
            camera: self
                .fallback_camera
                .unwrap_or_else(|| camera_centered(scene, (0.0, 0.0), self.screen_size)),
            player_marker: None,
            sprites: &[],
        };
        Some(RenderFrame {
            tilemaps: &[],
            planes: &[],
            sprites: &[],
            windows: &[],
            fallback: Some(fallback),
        })
    }

    fn rebuild_rgss_state(&mut self, textures: &HashMap<u32, Arc<RgbaImage>>) {
        self.tilemaps.clear();
        self.sprites.clear();
        self.planes.clear();
        self.windows.clear();
        let viewport_map = self.collect_viewports();
        let default_viewport = ViewportInfo::default(self.screen_size);
        self.collect_tilemaps(textures, &viewport_map, &default_viewport);
        self.collect_planes(textures, &viewport_map, &default_viewport);
        self.collect_sprites(textures, &viewport_map, &default_viewport);
        self.collect_windows(textures, &viewport_map, &default_viewport);
        self.tilemaps.sort_by_key(|tm| tm.z);
        self.planes.sort_by_key(|pl| pl.z);
        // Tie-break by creation order so SpriteWindow's 16 component sprites
        // (back, corners, sides, …, contents) stack in PE's intended order
        // when they all share the same z. Without this, HashMap-snapshot
        // iteration is non-deterministic and "back" can render OVER "contents",
        // hiding the message text.
        self.sprites.sort_by_key(|sp| (sp.z, sp.creation_order));
        self.windows.sort_by_key(|w| w.z);
    }

    fn collect_viewports(&self) -> HashMap<u32, ViewportInfo> {
        viewport_snapshot()
            .into_iter()
            .map(|(id, data)| {
                let clip = ClipRect::new(
                    data.rect.x,
                    data.rect.y,
                    data.rect.width.max(0) as u32,
                    data.rect.height.max(0) as u32,
                );
                let info = ViewportInfo {
                    rect: clip,
                    ox: data.ox as f32,
                    oy: data.oy as f32,
                    z: data.z,
                    visible: data.visible && !data.disposed,
                    tone: [
                        data.tone.red,
                        data.tone.green,
                        data.tone.blue,
                        data.tone.gray,
                    ],
                    color: [
                        data.color.red,
                        data.color.green,
                        data.color.blue,
                        data.color.alpha,
                    ],
                };
                (id, info)
            })
            .collect()
    }

    fn collect_tilemaps(
        &mut self,
        textures: &HashMap<u32, Arc<RgbaImage>>,
        viewports: &HashMap<u32, ViewportInfo>,
        default_viewport: &ViewportInfo,
    ) {
        for (_id, tilemap) in tilemap_snapshot() {
            if tilemap.disposed || !tilemap.visible {
                continue;
            }
            let Some(scene) = self.scene_from_tilemap(&tilemap, textures) else {
                continue;
            };
            let viewport = tilemap
                .viewport_id
                .and_then(|id| viewports.get(&id))
                .unwrap_or(default_viewport);
            if !viewport.visible {
                continue;
            }
            let clip = viewport
                .rect
                .clamp(self.screen_size)
                .unwrap_or_else(|| ClipRect::new(0, 0, self.screen_size.0, self.screen_size.1));
            let camera = Camera {
                origin: (
                    tilemap.ox as f32 + viewport.ox,
                    tilemap.oy as f32 + viewport.oy,
                ),
            };
            let tone = combine_tone(
                [
                    tilemap.tone.red,
                    tilemap.tone.green,
                    tilemap.tone.blue,
                    tilemap.tone.gray,
                ],
                viewport,
            );
            let color = combine_color(
                [
                    tilemap.color.red,
                    tilemap.color.green,
                    tilemap.color.blue,
                    tilemap.color.alpha,
                ],
                viewport,
            );
            self.tilemaps.push(TilemapInstance {
                scene,
                camera,
                clip,
                z: z_key(tilemap.viewport_id, viewport.z, 0),
                tone,
                color,
                opacity: tilemap.opacity.clamp(0, 255) as u8,
                blend_type: tilemap.blend_type.clamp(0, 2) as u8,
            });
        }
    }

    fn collect_planes(
        &mut self,
        textures: &HashMap<u32, Arc<RgbaImage>>,
        viewports: &HashMap<u32, ViewportInfo>,
        default_viewport: &ViewportInfo,
    ) {
        for (_id, plane) in plane_snapshot() {
            if plane.disposed || !plane.visible {
                continue;
            }
            let Some(bitmap) = plane.bitmap_id.and_then(|id| textures.get(&id).cloned()) else {
                continue;
            };
            let viewport = plane
                .viewport_id
                .and_then(|id| viewports.get(&id))
                .unwrap_or(default_viewport);
            if !viewport.visible {
                continue;
            }
            let clip = viewport
                .rect
                .clamp(self.screen_size)
                .unwrap_or_else(|| ClipRect::new(0, 0, self.screen_size.0, self.screen_size.1));
            let tone = combine_tone(
                [
                    plane.tone.red,
                    plane.tone.green,
                    plane.tone.blue,
                    plane.tone.gray,
                ],
                viewport,
            );
            let color = combine_color(
                [
                    plane.color.red,
                    plane.color.green,
                    plane.color.blue,
                    plane.color.alpha,
                ],
                viewport,
            );
            self.planes.push(PlaneInstance {
                texture: bitmap,
                clip,
                scroll: (plane.ox as f32 + viewport.ox, plane.oy as f32 + viewport.oy),
                zoom: (plane.zoom_x.max(0.0), plane.zoom_y.max(0.0)),
                opacity: plane.opacity.clamp(0, 255) as u8,
                blend_type: plane.blend_type.clamp(0, 2) as u8,
                tone,
                color,
                z: z_key(plane.viewport_id, viewport.z, plane.z),
            });
        }
    }

    fn collect_sprites(
        &mut self,
        textures: &HashMap<u32, Arc<RgbaImage>>,
        viewports: &HashMap<u32, ViewportInfo>,
        default_viewport: &ViewportInfo,
    ) {
        for (id, sprite) in sprite_snapshot() {
            if sprite.disposed || !sprite.visible {
                continue;
            }
            let Some(bitmap) = sprite.bitmap_id.and_then(|id| textures.get(&id).cloned()) else {
                continue;
            };
            let viewport = sprite
                .viewport_id
                .and_then(|id| viewports.get(&id))
                .unwrap_or(default_viewport);
            if !viewport.visible {
                continue;
            }
            let clip = viewport
                .rect
                .clamp(self.screen_size)
                .unwrap_or_else(|| ClipRect::new(0, 0, self.screen_size.0, self.screen_size.1));
            if clip.width == 0 || clip.height == 0 {
                continue;
            }
            let src_x = sprite.src_rect.x.max(0) as u32;
            let src_y = sprite.src_rect.y.max(0) as u32;
            let src_w = if sprite.src_rect.width > 0 {
                sprite.src_rect.width.max(0) as u32
            } else {
                bitmap.width()
            };
            let src_h = if sprite.src_rect.height > 0 {
                sprite.src_rect.height.max(0) as u32
            } else {
                bitmap.height()
            };
            if src_w == 0 || src_h == 0 {
                continue;
            }
            let position = (
                viewport.rect.x as f32 + (sprite.x - viewport.ox),
                viewport.rect.y as f32 + (sprite.y - viewport.oy),
            );
            // RGSS convention: ox/oy is the origin offset *within the source
            // rectangle*, not the entire bitmap. So bitmap pixel
            // (src_left + ox, src_top + oy) is anchored at (sprite.x,
            // sprite.y) on screen. The renderer's per-pixel formula expects
            // pivot = (ox, oy) so that:
            //   src_x = src_left + pivot_x + (dest_x - pos_x)
            //         = src_left + ox + (dest_x - pos_x)
            // Subtracting src_x here would double-count and shift each
            // animation frame on screen by its src_rect.x.
            let pivot = (sprite.ox as f32, sprite.oy as f32);
            let tone = combine_tone(
                [
                    sprite.tone.red,
                    sprite.tone.green,
                    sprite.tone.blue,
                    sprite.tone.gray,
                ],
                viewport,
            );
            let color = combine_color(
                [
                    sprite.color.red,
                    sprite.color.green,
                    sprite.color.blue,
                    sprite.color.alpha,
                ],
                viewport,
            );
            let flash_empty = sprite.flash.as_ref().map(|f| f.empty).unwrap_or(false);
            let flash_color = sprite.flash.as_ref().and_then(|flash| {
                if flash.empty {
                    None
                } else {
                    Some([
                        flash.color.red,
                        flash.color.green,
                        flash.color.blue,
                        flash.color.alpha,
                    ])
                }
            });
            self.sprites.push(SpriteInstance {
                texture: bitmap,
                src_rect: (src_x, src_y, src_w, src_h),
                origin: pivot,
                position,
                opacity: sprite.opacity.clamp(0, 255) as u8,
                z: z_key(sprite.viewport_id, viewport.z, sprite.z),
                creation_order: id,
                scale: (sprite.zoom_x, sprite.zoom_y),
                angle: sprite.angle,
                mirror: sprite.mirror,
                tone,
                color,
                blend_type: sprite.blend_type.clamp(0, 2) as u8,
                bush_depth: sprite.bush_depth.max(0) as u32,
                bush_opacity: sprite.bush_opacity.clamp(0, 255) as u8,
                clip,
                flash_color,
                flash_empty,
            });
        }
    }

    fn collect_windows(
        &mut self,
        textures: &HashMap<u32, Arc<RgbaImage>>,
        viewports: &HashMap<u32, ViewportInfo>,
        default_viewport: &ViewportInfo,
    ) {
        for (_id, window) in window_snapshot() {
            if window.disposed || !window.visible {
                continue;
            }
            let viewport = window
                .viewport_id
                .and_then(|id| viewports.get(&id))
                .unwrap_or(default_viewport);
            if !viewport.visible {
                continue;
            }
            let clip = viewport
                .rect
                .clamp(self.screen_size)
                .unwrap_or_else(|| ClipRect::new(0, 0, self.screen_size.0, self.screen_size.1));
            if clip.width == 0 || clip.height == 0 {
                continue;
            }
            let frame_rect = ClipRect::new(
                viewport.rect.x + window.x,
                viewport.rect.y + window.y,
                window.width.max(0) as u32,
                window.height.max(0) as u32,
            );
            if frame_rect.width == 0 || frame_rect.height == 0 {
                continue;
            }
            let visible_rect = match apply_openness(frame_rect, window.openness)
                .intersect(&clip)
                .and_then(|rect| rect.clamp(self.screen_size))
            {
                Some(rect) => rect,
                None => continue,
            };
            let windowskin = window
                .windowskin_id
                .and_then(|id| textures.get(&id).cloned());
            let contents = window.contents_id.and_then(|id| textures.get(&id).cloned());
            let cursor_rect = if window.cursor_rect.width > 0 && window.cursor_rect.height > 0 {
                let contents_x = frame_rect.x + WINDOW_PADDING - window.ox;
                let contents_y = frame_rect.y + WINDOW_PADDING - window.oy;
                Some(ClipRect::new(
                    contents_x + window.cursor_rect.x,
                    contents_y + window.cursor_rect.y,
                    window.cursor_rect.width.max(0) as u32,
                    window.cursor_rect.height.max(0) as u32,
                ))
            } else {
                None
            };
            let tone = combine_tone(
                [
                    window.tone.red,
                    window.tone.green,
                    window.tone.blue,
                    window.tone.gray,
                ],
                viewport,
            );
            let color = combine_color(
                [
                    window.color.red,
                    window.color.green,
                    window.color.blue,
                    window.color.alpha,
                ],
                viewport,
            );
            self.windows.push(WindowInstance {
                frame_rect,
                visible_rect,
                clip,
                windowskin,
                contents,
                contents_origin: (window.ox, window.oy),
                opacity: window.opacity.clamp(0, 255) as u8,
                back_opacity: window.back_opacity.clamp(0, 255) as u8,
                contents_opacity: window.contents_opacity.clamp(0, 255) as u8,
                tone,
                color,
                cursor_rect,
                cursor_active: window.active || window.pause,
                z: z_key(window.viewport_id, viewport.z, window.z),
            });
        }
    }

    fn collect_textures(&self) -> HashMap<u32, Arc<RgbaImage>> {
        bitmap_snapshot()
            .into_iter()
            .filter(|(_, data)| !data.disposed)
            .map(|(id, data)| (id, data.texture))
            .collect()
    }

    fn has_rgss_frame(&self) -> bool {
        !(self.tilemaps.is_empty()
            && self.planes.is_empty()
            && self.sprites.is_empty()
            && self.windows.is_empty())
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
        let flash_map = tilemap.flash.as_ref().and_then(|flash| {
            if flash.width == 0 || flash.height == 0 || flash.values.is_empty() {
                None
            } else {
                Some(TilemapFlashMap {
                    width: flash.width,
                    height: flash.height,
                    values: flash.values.clone(),
                })
            }
        });
        Some(TileScene {
            map_width: grid.width,
            map_height: grid.height,
            tile_size: 32,
            tileset,
            autotiles,
            layers: grid.layers.clone(),
            priorities,
            flash_map,
            flash_phase: tilemap.flash_phase,
        })
    }
}

#[derive(Clone, Copy)]
struct ViewportInfo {
    rect: ClipRect,
    ox: f32,
    oy: f32,
    z: i32,
    visible: bool,
    tone: [f32; 4],
    color: [f32; 4],
}

impl ViewportInfo {
    fn default(screen: (u32, u32)) -> Self {
        Self {
            rect: ClipRect::new(0, 0, screen.0, screen.1),
            ox: 0.0,
            oy: 0.0,
            z: 0,
            visible: true,
            tone: [0.0; 4],
            color: [0.0; 4],
        }
    }
}

fn camera_centered(scene: &TileScene, focus: (f32, f32), screen: (u32, u32)) -> Camera {
    let tile_px = scene.tile_size as f32;
    let map_px_w = scene.map_width as f32 * tile_px;
    let map_px_h = scene.map_height as f32 * tile_px;
    let viewport_w = screen.0 as f32;
    let viewport_h = screen.1 as f32;
    let focus_x = (focus.0 + 0.5) * tile_px;
    let focus_y = (focus.1 + 0.5) * tile_px;
    let max_x = (map_px_w - viewport_w).max(0.0);
    let max_y = (map_px_h - viewport_h).max(0.0);
    let origin_x = focus_x - viewport_w / 2.0;
    let origin_y = focus_y - viewport_h / 2.0;
    Camera {
        origin: (origin_x.clamp(0.0, max_x), origin_y.clamp(0.0, max_y)),
    }
}

fn combine_tone(mut tone: [f32; 4], viewport: &ViewportInfo) -> [f32; 4] {
    tone[0] = clamp_tone_channel(tone[0] + viewport.tone[0]);
    tone[1] = clamp_tone_channel(tone[1] + viewport.tone[1]);
    tone[2] = clamp_tone_channel(tone[2] + viewport.tone[2]);
    tone[3] = clamp_gray_channel(tone[3] + viewport.tone[3]);
    tone
}

fn combine_color(mut color: [f32; 4], viewport: &ViewportInfo) -> [f32; 4] {
    color[0] = clamp_color_channel(color[0] + viewport.color[0]);
    color[1] = clamp_color_channel(color[1] + viewport.color[1]);
    color[2] = clamp_color_channel(color[2] + viewport.color[2]);
    color[3] = clamp_color_channel(color[3] + viewport.color[3]);
    color
}

/// Combine viewport.z and sprite.z into a single sortable i64 key.
///
/// In RGSS, `Viewport.z` is the **primary** sort key — a sprite in a
/// high-z viewport is always above one in a low-z viewport, regardless of
/// the sprite's own z. The sprite's `z` is only the tie-breaker within the
/// same viewport.
///
/// PE uses `sprite.z = 99999` for message windows, far higher than the
/// 0–999 range our previous `viewport.z * 1000 + sprite.z` formula
/// assumed, which caused message windows in viewport.z=0 to sort below
/// scene pictures in viewport.z=200.
///
/// MULT = 1,000,000 keeps `viewport.z` strictly dominant for any
/// realistic sprite.z (PE max ≈ 99999, well under 1M).
fn combined_z(primary: i32, secondary: i32) -> i64 {
    const MULT: i64 = 1_000_000;
    (primary as i64) * MULT + (secondary as i64)
}

/// Compute the sort key for a sprite/plane/window given its viewport.
///
/// `viewport_id`: None if the sprite has no viewport (RGSS allows this
/// — drawn directly to the screen). When None, RGSS uses `sprite.z`
/// directly as the global sort key — equivalent to "viewport.z =
/// sprite.z" in our combined formulation.
fn z_key(viewport_id: Option<u32>, viewport_z: i32, sprite_z: i32) -> i64 {
    if viewport_id.is_some() {
        combined_z(viewport_z, sprite_z)
    } else {
        // Viewport-less sprite: its own z is the primary global ordering.
        combined_z(sprite_z, 0)
    }
}

fn clamp_tone_channel(value: f32) -> f32 {
    value.clamp(-255.0, 255.0)
}

fn clamp_gray_channel(value: f32) -> f32 {
    value.clamp(0.0, 255.0)
}

fn clamp_color_channel(value: f32) -> f32 {
    value.clamp(0.0, 255.0)
}

fn apply_openness(frame: ClipRect, openness: i32) -> ClipRect {
    let clamped = openness.clamp(0, 255) as u32;
    if clamped >= 255 {
        return frame;
    }
    if clamped == 0 || frame.height == 0 {
        return ClipRect::empty();
    }
    let visible_height = (frame.height as u64 * clamped as u64 / 255) as u32;
    if visible_height == 0 {
        return ClipRect::empty();
    }
    let trimmed = ((frame.height - visible_height) / 2) as i32;
    ClipRect::new(frame.x, frame.y + trimmed, frame.width, visible_height)
}
