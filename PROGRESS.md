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
- Added `rmxp-data` crate with a Marshal reader/JSON bridge to inspect `.rxdata`
  contents.

## 🚧 Immediate Goals

1. **Resource Loader Integration** – plug `rmxp-data` into `engine-core` and load
   real RMXP database/system files.
2. **Rendering Roadmap** – replace the gradient with tilemap + sprite batching
   using actual RMXP assets and Table data.
3. **Input & Loop** – encode fixed-timestep scheduling, keyboard/gamepad/touch
   mapping, and state machines for player movement.
4. **Audio Playback** – wrap rodio handles for BGM/BGS/ME/SE with fading, looping,
   and MIDI via `rustysynth`.
5. **RGSS Integration** – embed Ruby MRI, expose RGSS classes, and drive the
   scene stack via scripts.
6. **Mobile Shells** – add Swift/Kotlin launchers that delegate to the shared
   Rust engine.
