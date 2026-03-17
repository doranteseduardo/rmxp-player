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
  `Bitmap#get_pixel`/`set_pixel`, `Bitmap#clear`, and `Graphics.snap_to_bitmap`
  manipulate the same textures the renderer consumes, while script-level Cache
  logic is left entirely to project `Scripts.rxdata`.
- Added `rmxp-data` crate with a Marshal reader/JSON bridge plus engine wiring
  that reads `Data/System.rxdata`/`MapInfos.rxdata` from `RMXP_GAME_PATH`.
- Engine now parses the start map and feeds a rendered tile scene (tileset +
  autotiles) into pixels so we can visualize `.rxdata` content end-to-end.
- Renderer handles autotile sampling/animation, multi-layer composition, and
  RGSS priority tables so ground/overlay layers display correctly.
- Desktop runner hosts a fixed 60 Hz loop with keyboard input (arrows/WASD),
  pixel-perfect camera scroll (640×480 viewport), and a placeholder player
  marker to visualize movement on real maps.

## 🚧 Immediate Goals

1. **Scene Loop Integration** – execute `Scripts.rxdata`, drive the scene stack,
   and surface characters/windows via the renderer (now that tilemaps, planes,
   and sprites sync from Ruby).
2. **Audio Playback** – wrap rodio handles for BGM/BGS/ME/SE with fades, loop
   points, and MIDI via `rustysynth`.
3. **Event/Interpreter Core** – implement Game_Map/Game_Player logic hooked to
   passability, event triggers, and script callbacks.
4. **Persistence & Mobile Shells** – add save slots/config, then wire Swift/
   Kotlin launchers that reuse the Rust core on iOS/Android.
