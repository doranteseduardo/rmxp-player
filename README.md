# RMXP Native Player (Rust)

Ground-up, from-scratch implementation of the RPG Maker XP runtime that keeps the
original RGSS entry points but replaces the entire engine stack with modern,
cross-platform Rust crates.

---

## Feature Highlights

- **True RGSS Compatibility** – An embedded Ruby 3.2 (MRI) VM plus a native
  `RGSS::Native` bridge mirror the stock `Graphics`, `Bitmap`, `Viewport`,
  `Sprite`, `Plane`, `Window`, and `Tilemap` classes. Real `Scripts.rxdata`
  files run unmodified and control every on-screen object.
- **Multiplatform Rendering Loop** – A unified `winit + pixels (wgpu)` backend
  drives Metal (macOS/iOS), Vulkan/DX12 (Windows/Linux), and GLES/Vulkan
  (Android) with a strict 640×480 logical surface and pixel-perfect scaling.
- **Data-Driven Loading Pipeline** – The `rmxp-data` crate parses Marshal 4.8
  `.rxdata` content (`System`, `MapInfos`, maps, tilesets) so native code can
  bootstrap any project assets before Ruby takes over.
- **Audio + Platform Glue** – `audio` seeds the Rodio/CPAL stack, while the
  `platform` crate owns logging, config/save directories, and runtime settings.
  Mobile shells (Swift/Kotlin) embed the same Rust core via `winit`’s
  experimental mobile backends.

---

## Workspace Layout

```
Cargo.toml
apps/
  desktop-runner/        # Primary desktop binary (macOS/Windows/Linux)
crates/
  engine-core/           # Event loop, scheduler, scene bootstrap glue
  render/                # Pixels/wgpu renderer + tilemap/sprite/window compositing
  audio/                 # Rodio/CPAL audio subsystem (channels, fades, MIDI hooks)
  platform/              # Config/save dirs, tracing/log setup, env helpers
  data/ (rmxp-data)      # Marshal reader + typed RMXP structs
  rgss-bindings/         # Ruby MRI host + RGSS native bridge + primitives
  mobile-shell/          # iOS/Android launch helpers (future Swift/Kotlin entrypoints)
```

---

## Architecture Overview

| Layer            | Responsibility                                                                                      |
|------------------|------------------------------------------------------------------------------------------------------|
| `engine-core`    | Bootstraps the app (desktop/mobile), resolves project paths, feeds data into RGSS, and schedules the 60 Hz loop. |
| `rgss-bindings`  | Embeds Ruby (via `rb-sys`), exposes native handles, and evaluates `Scripts.rxdata`.                  |
| `render`         | Converts RGSS snapshots (tilemaps, sprites, planes, windows, screen effects) into GPU-ready frames.  |
| `audio`          | Sets up the Rodio sink / CPAL streams for BGM, BGS, ME, and SE playback (loop points + fades).       |
| `data`           | Parses `.rxdata` via a Marshal reader so boot code can inspect `System`, `MapInfos`, `Tilesets`, etc. |
| `platform`       | Logging, config/save/storage paths (`RMXP_LOG`, app support dirs), basic CLI helpers, env overrides. |
| `mobile-shell`   | Lightweight native launchers that embed the event loop on iOS (UIKit + Metal) and Android (Activity + SurfaceView). |

---

## Status Matrix

| Area                          | Status | Notes |
|-------------------------------|--------|-------|
| Cargo workspace / tooling     | ✅      | Multi-crate layout with `cargo workspace` + shared `clippy`/`fmt` config. |
| Graphics built-ins            | ✅      | `Graphics`, `Bitmap`, `Viewport`, `Sprite`, `Plane`, `Window`, `Tilemap` mirror RGSS behavior, including tone/flash, windowskins, zoom/rotation/mirror, autotile animation, etc. |
| RGSS data loading             | ✅      | `rmxp-data` reads Marshal 4.8 for `System`, `MapInfos`, map layers, tilesets/priorities. |
| Tilemap renderer              | ✅      | Priorities, autotiles, multi-layer composition, per-map scroll/viewport handling. |
| RGSS-driven scene rendering   | ✅      | Renderer consumes live snapshots from Ruby (sprites, planes, windows) or falls back to native tile scenes. |
| Ruby embedding                | ✅      | MRI 3.2 via `rb-sys`; `Scripts.rxdata` executes in-process with native extension hooks. |
| Audio playback                | 🚧      | Rodio/CPAL streams initialize, mixer/channel controls are being wired to RGSS `Audio.*`. |
| Scene loop / interpreter      | 🚧      | Need to drive Game_Map/Game_Player/Game_Interpreter from Ruby to move past static demos. |
| Persistence & mobile shells   | 🚧      | Save/config plumbing + Swift/Kotlin wrappers are stubs awaiting implementation. |

---

## Requirements

| Tool / Library | Notes |
|----------------|-------|
| Rust 1.75+     | Install via [rustup](https://rustup.rs/) (MSRV follows stable). |
| Ruby 3.2 (MRI) | Required for `rb-sys`. Install via rbenv/rvm/RVM/Homebrew/RVM, then expose via env vars. |
| Ruby dev libs  | Ruby headers + `libruby` must be discoverable. Homebrew installs place them under `/opt/homebrew/opt/ruby@3.2`. |
| Platform deps  | Metal SDK (macOS/iOS), Vulkan/DirectX drivers (Windows/Linux), Android NDK (for mobile builds). |

Set up Ruby for `rb-sys`:

```bash
export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby   # adjust to your install
```

---

## Building & Running (Desktop)

```bash
git clone https://github.com/your-org/rmxp-native-player.git
cd rmxp-native-player

# point to your Ruby install (see above)
export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby

# point to the RMXP project (folder that contains Data/, Graphics/, Audio/, etc.)
export RMXP_GAME_PATH=/absolute/path/to/your/project

# optional: boot a specific map ID instead of the System.rxdata start map
export RMXP_START_MAP=2

cargo run -p desktop-runner
```

Environment variables accepted by the desktop runner:

| Variable          | Default / Purpose |
|-------------------|------------------|
| `RMXP_GAME_PATH`  | **Required.** Absolute path to the RMXP project root (expects `Data/System.rxdata`). |
| `RMXP_START_MAP`  | Optional. Integer map ID override (falls back to `System.rxdata` if missing/invalid). |
| `RMXP_LOG`        | Logging filter (e.g. `RMXP_LOG=debug` to enable verbose tracing). |

The runner opens a 640×480 window, parses the target project’s map database,
boots Ruby, evaluates every script section, and mirrors the resulting RGSS
objects into the renderer. When Ruby isn’t driving anything yet, the engine
falls back to the native tile scene so you can still visualize assets.

---

## Development Tips

| Task                 | Command |
|----------------------|---------|
| Format all crates    | `cargo fmt` |
| Lint (workspace)     | `cargo clippy --all-targets --all-features` |
| Desktop smoke run    | `RMXP_GAME_PATH=… cargo run -p desktop-runner` |
| Check without running| `cargo check` |

Extra tips:

- Set `RMXP_LOG=debug` to watch data loading, RGSS snapshots, and renderer stats.
- Use `RMXP_START_MAP=<id>` when testing specific maps (e.g., Pokémon Essentials
  outdoor vs indoor scenes).
- The renderer captures every backbuffer; `Graphics.snap_to_bitmap` returns a
  real `Bitmap` (saved in the RGSS handle store) so vanilla screenshot code
  works.

---

## Roadmap

1. **Scene Loop Integration** – Execute `Main`, wire Game_Map/Game_Player, and
   run the interpreter so player movement/events/UI flow originate entirely in
   Ruby.
2. **Audio Playback** – Implement RGSS `Audio.*` channels on top of Rodio/CPAL
   (loop points, fades, ME ducking, MIDI via `rustysynth` with bundled
   SoundFonts).
3. **Event/Interpreter Core** – Native helpers for passability, collision, and
   message/input handling that Ruby can call into for performance-sensitive
   operations.
4. **Persistence & Config** – Save slots, config options (resolution, control
   remaps, touch overlays), and sandbox-safe storage across desktop/mobile.
5. **Mobile Shells** – Ship Swift/Kotlin launchers that expose document pickers,
   lifecycle callbacks, sandboxed storage, and background audio permissions.

Contributions are welcome—open an issue or PR with the subsystem you’d like to
tackle, and include details about your RMXP project/setup so we can reproduce. !*** End Patch***}
***
End Patch
