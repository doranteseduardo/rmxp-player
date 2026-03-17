# RMXP Native Player (Rust)

Ground-up reimplementation of the RPG Maker XP runtime using a native Rust stack.
The project is organized as a Cargo workspace with focused crates for the core
engine, rendering, audio, platform utilities, RGSS bindings, and future mobile shells.

## Current Status

- ✅ Workspace scaffolding with crates for `engine-core`, `render`, `audio`,
  `platform`, `rgss-bindings`, `mobile-shell`, and `desktop-runner` binary.
- ✅ Winit + Pixels desktop loop rendering a placeholder gradient.
- ✅ Rodio audio backend initialization stub.
- ✅ Platform helper for config directories and logging bootstrap.
- ✅ `rmxp-data` crate with a Marshal (Ruby 4.8) parser + JSON helpers.
- 🚧 Pending: real RGSS embedding, map renderer, input mapping, event system,
  audio playback, save/load, and mobile launchers.

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
cargo run -p desktop-runner
```

Environment variables:

- `RMXP_LOG=debug` – increases log verbosity (uses `tracing-subscriber`).

## Next Steps

1. Flesh out `rgss-bindings` with embedded Ruby MRI bootstrap and native class
   shims for RGSS.
2. Replace the placeholder gradient renderer with tilemap/sprite rendering backed
   by real RMXP assets.
3. Implement resource loading, filesystem abstractions, and project selection UI.
4. Expand `platform` crate for mobile-safe paths and asynchronous file pickers.
5. Add mobile shells (Swift/Kotlin) leveraging the shared Rust engine via winit.
