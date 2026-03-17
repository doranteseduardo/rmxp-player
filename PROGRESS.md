# Development Progress

## ✅ Bootstrapped

- Workspace migrated to a multi-crate layout (`engine-core`, `render`, `audio`,
  `platform`, `rgss-bindings`, `mobile-shell`, `desktop-runner`).
- Desktop runner crate launches a winit event loop and renders a placeholder
  frame via the shared renderer abstraction.
- Audio subsystem initializes the default rodio output stream (no playback yet).
- Platform helper configures config/save directories and installs `tracing`
  logging with `RMXP_LOG` filtering.
- RGSS bindings crate contains a stub `RubyVm` placeholder for the future MRI
  embedding layer.
- Added `rmxp-data` crate with a Marshal reader/JSON bridge plus engine wiring
  that reads `Data/System.rxdata`/`MapInfos.rxdata` from `RMXP_GAME_PATH`.
- Engine now parses the start map and feeds a rendered tile scene (tileset +
  autotiles) into pixels so we can visualize `.rxdata` content end-to-end.
- Renderer handles autotile sampling/animation, multi-layer composition, and
  RGSS priority tables so ground/overlay layers display correctly.
- Desktop runner hosts a fixed 60 Hz update loop with keyboard input (arrows or
  WASD) driving a placeholder player marker for movement debugging.

## 🚧 Immediate Goals

1. **Audio Playback** – wrap rodio handles for BGM/BGS/ME/SE with fading, looping,
   and MIDI via `rustysynth`.
2. **RGSS Integration** – embed Ruby MRI, expose RGSS classes, and drive the
   scene stack via scripts.
3. **Event/Interpreter Core** – implement Game_Map/Game_Player logic hooked to
   passability, event triggers, and future script calls.
4. **Mobile Shells** – add Swift/Kotlin launchers that delegate to the shared
   Rust engine.
