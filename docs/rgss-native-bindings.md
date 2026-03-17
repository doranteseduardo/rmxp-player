# RGSS Native Bindings Design

This document captures the final design for the remaining RGSS classes that must
be implemented natively (mirroring mkxp-z) so Ruby scripts interact with engine
handles instead of Ruby-only shims. Every class here will be registered from
`rgss-bindings/src/classes` using `rb_define_class`/`rb_define_alloc_func` and a
typed-data payload that carries the engine handle IDs.

## Shared Patterns

| Topic | Decision |
|-------|----------|
| Ruby object layout | Each class becomes a Ruby `TypedData` whose `data` pointer stores a `struct HandleRef { u32 id; bool disposed; }`. Garbage collection invokes a common `dispose_handle` helper that calls the matching native function if the script did not call `dispose` manually. |
| Handle stores | The existing stores in `native::{bitmap, viewport, sprite, plane, window, tilemap}` remain the single source of truth; Ruby objects never embed actual textures or data blobs. |
| Method surface | Use mkxp-z’s public API as the checklist. Anything that mkxp-z exposes must be wired 1:1 (argument count, default semantics, errors). Methods that were previously implemented in Ruby (`primitives.rb`) simply forward to the new C/Rust functions. |
| Type conversions | Centralize conversions inside `classes/common.rs` (e.g., `value_to_color`, `value_to_rect`, `value_to_table`). These helpers wrap the existing `value_to_*` utilities from `native::util`. |
| Dependency graph | Viewport/Sprite/Plane/Window/Tilemap objects store other handles (e.g., bitmap/viewports). Ruby setters look up the child object’s handle and pass IDs to the existing native setters. |
| Thread safety | All interactions stay on the main thread; HandleStore already protects access with mutexes. Ruby methods simply translate arguments and call into the existing native module functions (which mutate the stores). |

## Class Designs

### Color

- **Storage:** `struct ColorData { f32 red, green, blue, alpha }` (already defined).
- **Ruby binding:** `rb_define_class("Color", rb_cObject)` with typed data pointing
  to a heap-allocated `ColorData`.
- **Methods:** `initialize`, `set`, component accessors (reader/writer), `==`,
  `dup`, `to_a`. Writers clamp to `[0.0, 255.0]` to match RGSS.
- **Native use:** `color_to_rgba32(ColorData)` helper feeds existing sprite/window
  setters so no other module changes needed.

### Tone

- Same pattern as `Color` but with `.gray` instead of alpha. `set`, component
  accessors, `==`, `dup`.
- Add `tone_to_vec4` helper reused by viewport/sprite/window/tilemap setters.

### Rect

- `RectData` already exists; wrap it in typed data.
- Methods: `initialize`, `set`, accessors, `empty`, `==`, `dup`, `width=`/`height=`
  that clamp to `i32`.
- Provide `rect_to_components` helper for the sprite/src_rect and window/cursor_rect setters.

### Table

- **Storage:** Rust struct owning a `Vec<i16>` plus `xsize`, `ysize`, `zsize`.
- `initialize(x, y = 1, z = 1)`, `resize`, `[]`, `[]=`, `clone`, `dup`,
  `xsize/ysize/zsize` readers, and `@data.pack('s<*')` equivalent implemented as
  `fn pack_to_le_i16() -> VALUE` (Ruby string).
- Provide `Table::from_raw(bytes, x, y, z)` for MKXP compatibility when reading
  marshal data later.

### Font

- Keep static defaults inside Rust (`static FONT_DEFAULTS: Mutex<…>`).
- Instance fields: `Vec<String> name`, `i32 size`, `bool bold/italic/shadow`,
  `ColorData color`.
- Methods: `initialize(name = nil, size = nil)`, readers/writers for each field,
  `.default_*` class accessors referencing the shared defaults, `color=` storing
  a cloned `Color`.
- Bridge to renderer later (e.g., `bitmap_draw_text` reads font data from the
  owning bitmap object).

### Bitmap

- Replace Ruby class with native `Bitmap` typed data.
- Data stored in HandleStore (`BitmapData`) already exists. Ruby object only owns
  the handle ID.
- Methods map 1:1 to existing `RGSS::Native.bitmap_*` entry points:
  - `initialize(width, height)` or `initialize(path)`
  - `dispose`, `disposed?`, `width`, `height`, `rect`, `hue_change`
    (still TODO), `blt`, `stretch_blt`, `fill_rect`, `gradient_fill_rect`,
    `clear`, `text_size`, `draw_text`, `get_pixel`, `set_pixel`, `dup`.
- Each method fetches the handle ID from the typed data and calls the native
  function; packed color conversions move to shared helpers.
- Track association with `Font`: Ruby `Bitmap#font` returns a `Font` object
  stored alongside the handle (no native change needed yet; struct on Ruby side).

### Viewport

- Typed data storing `u32 viewport_id`.
- `initialize(x, y, width, height)` => call `RGSS::Native.viewport_create`.
- Setters/getters (`rect`, `rect=`, `visible`, `z`, `ox`, `oy`, `color`, `tone`).
- Dispose finalizer releases the handle once.
- When we snapshot for rendering we already read from the HandleStore, so the
  Ruby class just needs to ensure every mutation forwards to the `viewport_set_*`
  functions.

### Sprite

- Typed data storing:
  - `u32 sprite_id`
  - Weak references (`VALUE`) to the owning `Viewport` and `Bitmap` Ruby objects
    so GC keeps them alive as long as the sprite references them.
- Methods replicate mkxp-z:
  `initialize(viewport = nil)`, `viewport`, `viewport=`, `bitmap`, `bitmap=`,
  `x/y/z`, `ox/oy`, `zoom_x/zoom_y`, `angle`, `mirror`, `bush_depth`,
  `bush_opacity`, `opacity`, `blend_type`, `visible`, `src_rect`, `color`,
  `tone`, `flash`, `update`, `width`, `height`.
- Each setter updates cached Ruby fields (for fast `attr_reader`) and immediately
  calls the native setter.
- Disposal clears cached Ruby references and marks the native handle disposed.

### Plane

- Nearly identical to Sprite but without `src_rect` and bush fields. Maintains
  `bitmap`, `viewport`, `ox/oy`, `zoom_x/zoom_y`, `opacity`, `blend_type`,
  `visible`, `tone`, `color`.
- Uses `RGSS::Native.plane_*` functions already available.

### Window

- Typed data with `window_id`, plus VALUE references for `viewport`, `windowskin`,
  `contents`, `cursor_rect`, `tone`, `color`.
- Methods: `initialize(x = 0, y = 0, width = 32, height = 32, viewport = nil)`,
  attr readers/writers for all documented RGSS fields (`x`, `y`, `z`, `ox`, `oy`,
  `width`, `height`, `viewport`, `windowskin`, `contents`, `opacity`,
  `back_opacity`, `contents_opacity`, `openness`, `visible`, `active`, `pause`,
  `tone`, `color`, `cursor_rect`), plus `open`, `close`, `update`.
- Writers ensure the Ruby-side `Rect`/`Tone`/`Color` typed data are cloned when
  necessary to avoid accidental aliasing, but the actual pixel work is still
  performed via the `window_set_*` native functions.

### Tilemap

- Typed data with `tilemap_id`, references to `viewport`, `tileset`, `autotile`
  array, `Table` objects for `map_data`, `priorities`, `flash_data`, and `Tone`
  / `Color`.
- Methods: `initialize(viewport = nil)`, `dispose`, `disposed?`, `viewport`,
  `viewport=`, `tileset`, `tileset=`, `autotiles`, `autotiles=`, `bitmaps` alias,
  `map_data`, `map_data=`, `flash_data`, `flash_data=`, `priorities`,
  `priorities=`, `ox/oy`, `visible`, `opacity`, `blend_type`, `tone`, `color`,
  `update`.
- `autotiles=` iterates over the array and forwards IDs to
  `tilemap_set_autotile(index, handle)` while storing Ruby references for GC.
- Table setters validate dimensions before calling `tilemap_set_*` so corrupted
  data surfaces as Ruby `ArgumentError`, matching mkxp-z.

### Graphics Module (adjacent work)

- Already native, but once the classes above are native we can delete the Ruby
  wrappers for `Graphics.snap_to_bitmap` (`_native_wrap`).

## Implementation Steps

1. **Common infrastructure**
   - Create `classes/mod.rs` with shared macros for typed-data allocation,
     disposal, and method definition.
   - Move `primitives.rb` responsibilities into Rust incrementally; gate with a
     feature flag (`rgss_primitives_fallback`) until all classes are ported.
2. **Simple value classes first** (`Color`, `Tone`, `Rect`, `Table`, `Font`) so
   the complex objects can reuse them.
3. **Resource-backed classes** in order of dependency:
   1. `Bitmap` (needs `Color`, `Rect`, `Font` ready).
   2. `Viewport`
   3. `Sprite` + `Plane`
   4. `Window`
   5. `Tilemap`
4. **Shim removal** – delete the corresponding Ruby class definitions from
   `primitives.rb` once the native bindings pass tests. Keep compatibility guards
   that raise `NotImplementedError` if someone tries to load without the native
   bridge compiled.
5. **Testing** – add unit tests per class (instantiate, set fields, confirm
   snapshots match expected data) and integration tests loading sample RMXP maps.

With this design, the RGSS surface area matches mkxp-z’s expectations and every
Ruby-visible object now directly controls the engine’s native state.
