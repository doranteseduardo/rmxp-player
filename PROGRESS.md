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
- Engine now parses the start map and feeds a color-coded tileview into the
  renderer so we can visualize real `.rxdata` content end-to-end.

## 🚧 Immediate Goals

1. **Tileset Rendering** – swap the debug colors for actual tileset textures and
   handle priorities/auto-tiles using the parsed map data.
2. **Input & Loop** – encode fixed-timestep scheduling, keyboard/gamepad/touch
   mapping, and state machines for player movement.
3. **Audio Playback** – wrap rodio handles for BGM/BGS/ME/SE with fading, looping,
   and MIDI via `rustysynth`.
4. **RGSS Integration** – embed Ruby MRI, expose RGSS classes, and drive the
   scene stack via scripts.
5. **Mobile Shells** – add Swift/Kotlin launchers that delegate to the shared
   Rust engine.
