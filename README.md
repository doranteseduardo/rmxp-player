# RMXP Native Player (Rust)

Ground-up reimplementation of the RPG Maker XP runtime using a native Rust stack.
The project is organized as a Cargo workspace with focused crates for the core
engine, rendering, audio, platform utilities, RGSS bindings, and future mobile shells.

## Current Status

- ✅ Cargo workspace scaffolded (`engine-core`, `render`, `audio`, `data`,
  `platform`, `rgss-bindings`, `mobile-shell`, `desktop-runner`).
- ✅ Desktop runner boots a `winit` + `pixels` loop with Metal/Vulkan/DirectX
  backends and renders real RMXP tilemaps (tilesets + autotiles) pulled from
  `System.rxdata`, respecting priorities and animated quads.
- ✅ Rodio/CPAL audio system initializes (playback hooks pending).
- ✅ Platform helper configures config/save directories and installs `tracing`
  logging (enable verbose logs via `RMXP_LOG=debug`).
- ✅ `rmxp-data` crate parses Marshal 4.8 `.rxdata` files (`System`, `MapInfos`,
  maps, tilesets) and feeds typed structs to the engine.
- ✅ Embedded Ruby 3.2 VM via `rb-sys`, plus an `RGSS::Native` bridge that
  mirrors Bitmap/Viewport/Sprite/Window/Plane/Tilemap classes so Ruby owns scene
  objects while Rust keeps authoritative state for rendering.
- ✅ Core Graphics/Bitmap APIs are backed by native code: `Bitmap#blt`,
  `Bitmap#fill_rect`, `Bitmap#get_pixel`/`set_pixel`, and `Graphics.snap_to_bitmap`
  all operate on shared GPU-ready textures pulled straight from the renderer.
- ✅ Kernel helpers (`load_data`, `save_data`, `data_exist?`) resolve paths
  against the active project root so vanilla RGSS scripts can Marshal `.rxdata`
  content without any changes.
- ✅ Input loop maps WASD/arrow keys into an RGSS-style snapshot so the renderer
  can visualize scrolling/clamping at 640×480 (1:1 pixels, centered player).
- ✅ Tilemap data written by real RGSS scripts (tileset, autotiles, priorities,
  scroll offsets) now feeds the renderer, so Ruby code controls which map is
  displayed instead of the fixed `.rxdata` bootstrap scene.
- 🚧 Next: wire native RGSS state into the renderer (sprites/windows), implement
  audio channels, drive the scene stack from real scripts, add persistence, and
  stand up iOS/Android shells.

## Project Layout

```
Cargo.toml
apps/
  desktop-runner/      # Binary that boots the engine for desktop platforms
crates/
  engine-core/         # Event loop, scheduler skeleton, integration glue
  render/              # Pixels-based renderer abstraction
  audio/               # Rodio/CPAL audio system stub
  platform/            # Config directories, logging, persistence helpers
  data/                # Marshal reader + typed Ruby value utilities
  rgss-bindings/       # Placeholder Ruby/RGSS bridge
  mobile-shell/        # Future iOS/Android launch helpers
```

## Running (Desktop)

```bash
RMXP_GAME_PATH=/absolute/path/to/rmxp/game cargo run -p desktop-runner
```

Environment variables:

- `RMXP_GAME_PATH` – absolute path to the RMXP project folder (expects `Data/System.rxdata`).
- `RMXP_START_MAP` – optional override for the map ID to boot (defaults to `System.rxdata` start map).
- `RMXP_LOG=debug` – increases log verbosity (uses `tracing-subscriber`).

Ruby (MRI) embedding:

- Enable the optional `rgss-bindings/mri` feature to boot a real Ruby 3.2 VM:
  `cargo run -p desktop-runner --features rgss-bindings/mri`.
- You need Ruby 3.2 headers/libraries available to the build (install Ruby 3.2
  and expose it via `RB_SYS_RUBY_VERSION=3.2`, or point `libruby` via `RUBY` env).
  Without the feature the engine keeps using the stub VM for development.

Ruby dependency:

- A system Ruby 3.2 (MRI) toolchain with headers/libs is required. Install via
  `rbenv`, `ruby-install`, Homebrew (`brew install ruby@3.2`), etc.
- Tell `rb-sys` which Ruby to use, e.g.:

  ```bash
  export RB_SYS_RUBY_VERSION=3.2
  # optional: specify the exact ruby executable
  export RUBY=/opt/homebrew/opt/ruby@3.2/bin/ruby
  ```

- Without these env vars pointing to a valid Ruby 3.2 install, the build will
  fail before launching the engine.

Window & camera:

- Default window size is 640×480 to match vanilla RMXP. The renderer keeps a 1:1
  pixel scale and centers the player while clamping scroll at map edges.

Controls:

- Arrow keys or WASD – move the debug player marker across the tilemap.
- Enter/Space – placeholder confirm button (logged for future UI hooks).

## Next Steps

1. **RGSS Scene Loop** – execute real `Scripts.rxdata`, populate sprite/window
   registries, and drive the renderer with live Ruby scene stacks (characters,
   maps, UI).
2. **Audio Playback** – hook RGSS `Audio.*` calls to rodio (BGM/BGS/ME/SE, fades,
   MIDI via `rustysynth`).
3. **Event Interpreter** – implement Game_Map/Game_Player + interpreter loops to
   mirror RMXP behavior (messages, move routes, encounters, triggers).
4. **Persistence & Config** – save slots, config values (resolution, control
   mapping, audio levels), and mobile-friendly sandboxes/pickers.
5. **Mobile Shells** – add Swift/Kotlin launchers that embed the Rust core via
   `winit` mobile backends (document pickers, lifecycle, storage access).
