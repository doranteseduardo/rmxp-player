# Development Progress

## ✅ Bootstrapped

- Workspace migrated to a multi-crate layout (`engine-core`, `render`, `audio`,
  `platform`, `rgss-bindings`, `mobile-shell`, `desktop-runner`).
- Desktop runner crate launches a winit event loop and renders actual RMXP
  tilemaps via the shared renderer abstraction (Metal on macOS, Vulkan/DX on
  Linux/Windows).
- Audio subsystem initializes the default rodio output stream (no playback yet).
- Platform helper configures config/save directories and installs `tracing`
  logging with `RMXP_LOG` filtering.
- RGSS bindings crate boots an embedded Ruby 3.2 VM via `rb-sys`; building now
  requires a Ruby toolchain (`RB_SYS_RUBY_VERSION=3.2`, `RUBY=/path/to/ruby`).
- `RGSS::Native` bridge mirrors `Bitmap`, `Viewport`, `Sprite`, and `Window`
  classes so Ruby scripts manipulate the canonical state that Rust snapshots for
  rendering/audio subsystems.
- Plane and Tilemap RGSS classes now map to native handle stores; Ruby assigns
  tilesets/autotiles/priorities via `Table` blobs and the renderer rebuilds the
  map each frame using those live snapshots (ox/oy scroll respected).
- Kernel helpers (`load_data`, `save_data`, `data_exist?`) resolve project paths
  and reuse Ruby `Marshal`, so vanilla scripts can read/write `.rxdata` without
  touching the Rust side.
- Graphics/Bitmap built-ins now run natively: `Bitmap#blt`, `Bitmap#fill_rect`,
  `Bitmap#get_pixel`/`set_pixel`, `Bitmap#clear`, plus `Graphics.freeze`,
  `Graphics.snap_to_bitmap`, and the tone/brightness/flash pipeline manipulate
  the same textures the renderer consumes, while script-level Cache logic is
  left entirely to project `Scripts.rxdata`. Hue shifts (`Bitmap#hue_change`)
  and per-sprite flashes now route through the native renderer instead of Ruby
  shims.
- `Bitmap#stretch_blt`, `Bitmap#gradient_fill_rect`, and `Bitmap#draw_text` now
  render through a built-in 8×8 ASCII font so the standard RMXP windows and UI
  code can draw text/gradients without modification.
- Added `rmxp-data` crate with a Marshal reader/JSON bridge plus engine wiring
  that reads `Data/System.rxdata`/`MapInfos.rxdata` from `RMXP_GAME_PATH`.
- Engine now parses the start map and feeds a rendered tile scene (tileset +
  autotiles) into pixels so we can visualize `.rxdata` content end-to-end.
- Renderer handles autotile sampling/animation, multi-layer composition, and
  RGSS priority tables so ground/overlay layers display correctly.
- Desktop runner hosts a fixed 60 Hz loop with keyboard input (arrows/WASD),
  pixel-perfect camera scroll (640×480 viewport), and a placeholder player
  marker to visualize movement on real maps.
- RGSS viewports, sprites, planes, and windows now composite directly in the
  renderer (viewport clipping, sprite zoom/angle/mirror, tone/color overlays,
  plane tiling, windowskin/background/contents/cursor rendering), enabling
  vanilla `Scripts.rxdata` scenes to drive the entire frame without native
  stubs.
- Window close events no longer abort the process; instead they request a
  `Graphics` hangup that raises the RGSS `Hangup` exception on the next
  `Graphics.update`, matching mkxp-z’s lifecycle semantics.
- The extended mkxp-z `System` APIs (platform/OS queries, CPU & memory info,
  CSV parsing, launcher helpers, default font family, file existence checks) are
  implemented, so projects that lean on those helpers no longer crash during
  boot.
- Input now mirrors mkxp-z’s helpers: `release?`, `count`, `time?`, mouse coords,
  scroll deltas, clipboard/text input, and controller namespace stubs all exist,
  with blur/tone buttons feeding the native renderer instead of Ruby shims.
- `Graphics.blur`/`Graphics.sharpen` hook into the renderer’s screen-effects
  pass (with frozen-frame fallbacks) so transitions that rely on those filters
  match the RPG Maker runtime instead of silently no-oping.
- Tilemap flash tables now flow from Ruby `flash_data` to the renderer with the
  mkxp-z pulse curve, opacity clamping, and additive blend, while window cursor
  rectangles respect `ox/oy` so scrolled contents keep their highlight and tone/
  color operations mirror the reference implementation.

## 🚧 Immediate Goals

1. **Scene Loop Integration** – execute `Scripts.rxdata`, drive the RGSS scene
   stack (Game_Map/Game_Player/Game_Interpreter), and let Ruby advance maps,
   characters, and UI transitions end-to-end.
2. **Audio Playback** – wire RGSS `Audio.*` calls to the rodio/CPAL backend with
   fades, loop points, and MIDI via `rustysynth`.
3. **Event/Interpreter Core** – implement map passability, event triggers,
   message windows, and script callbacks so vanilla events run unchanged.
4. **Persistence & Mobile Shells** – add save slots/config, then wire Swift/
   Kotlin launchers that reuse the Rust core on iOS/Android.
