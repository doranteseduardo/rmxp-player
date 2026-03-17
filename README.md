# RMXP Native Player

Rust-based reimplementation of the RPG Maker XP runtime that preserves the
original RGSS entry points (Ruby scripts, Graphics/Bitmap/etc.) while replacing
the underlying renderer, audio stack, and platform glue with modern multiplatform
crates. The long-term goal is 1:1 functional parity with mkxp-z across desktop
and mobile targets.

---

## High-Level Components

| Layer | Purpose |
|-------|---------|
| `engine-core` | Boot flow, project discovery, fixed-step scheduler, platform lifecycle hooks. |
| `rgss-bindings` | Embeds Ruby MRI 3.2 via `rb-sys`, exposes native RGSS classes, evaluates `Scripts.rxdata`. |
| `render` | `winit + pixels (wgpu)` renderer for tilemaps, sprites, planes, windows, and screen effects. |
| `audio` | Rodio/CPAL plumbing for future RGSS `Audio.*` playback (channels, fades, loop points). |
| `rmxp-data` | Marshal 4.8 reader and typed RMXP structs (`System`, `MapInfos`, maps, tilesets). |
| `platform` | Config/save directories, logging (`tracing`), CLI/env helpers. |
| `apps/desktop-runner` | macOS/Windows/Linux binary that wires everything together. Mobile shells are staged next. |

---

## Current Capabilities

- **Workspace Foundations** – Cargo workspace with dedicated crates per subsystem,
  shared lint/format tooling, and a desktop runner binary that boots straight
  into a winit event loop.
- **Project Loading** – `rmxp-data` parses `Data/System.rxdata`, `MapInfos.rxdata`,
  tilesets, maps, and tileset metadata from any RMXP project (set via
  `RMXP_GAME_PATH`). `RMXP_START_MAP` overrides the starting map for testing.
- **Renderer** – Tile scenes render via wgpu/Metal with pixel-perfect 640×480
  output, autotile animation, and layer priorities. Viewports, sprites, planes,
  and windows are mirrored from Ruby handle stores when available.
- **Ruby Embedding** – MRI 3.2 boots in-process, evaluates real `Scripts.rxdata`
  sections, and bridges to native handles (System/Graphics/Bitmap/etc.).
- **Graphics Built-ins** – All RGSS built-in classes now run as native typed
  data. `Bitmap#hue_change`, `Bitmap#stretch_blt`, `Bitmap#draw_text`, and
  `Sprite#flash` all drive the renderer directly, so scripts no longer depend on
  Ruby fallbacks for visual effects. Screen-level `Graphics.blur`/`sharpen`
  now run through the renderer’s CPU post-process pipeline (or directly on the
  frozen frame), matching mkxp-z transitions.
- **Lifecycle Hooks** – Window close/destroy events trigger the RGSS `Hangup`
  exception instead of aborting the process, giving scripts a chance to intercept
  shutdown just like in mkxp-z.
- **System Utilities** – Platform detection (`System.platform`, `System.is_*?`),
  CPU/memory stats, CSV parsing, launcher helpers, and file probes are wired up,
  so scripts that rely on mkxp-z’s extended `System` module continue to run.
- **Input Parity** – `Input.release?`, `Input.count`, `Input.time?`, mouse
  coordinates, scroll deltas, text input, clipboard access, and the controller
  namespace are all available, feeding the same handle store the renderer sees
  instead of being patched in Ruby.
- **Diagnostics** – `tracing` logs project resolution, data parsing, renderer
  state, and Ruby evaluation status with `RMXP_LOG=debug`.

---

## Active Work & Known Gaps

1. **RGSS Built-ins 1:1** – Finish migrating the remaining value/resource
   classes to native handles, remove the stop-gap Ruby implementations, and
   audit method-by-method parity (tone ranges, window cursors, tilemap
   priorities, disposed? semantics, blur/sharpen, etc.).
2. **System/Input Glue** – Essentials still expects a few more `System.*`
   helpers plus complete input/device mappings. These APIs need to match mkxp-z
   before most projects can boot.
3. **Scene Loop & Interpreter** – Game_Map/Game_Player/Game_Interpreter wiring
   still needs to hand control to Ruby so player movement, events, and menus run
   end-to-end.
4. **Audio** – Rodio/CPAL streams initialize, but actual BGM/BGS/ME/SE playback,
   fades, and MIDI are pending.
5. **Persistence & Mobile Shells** – Save/config storage plus Swift/Kotlin
   launchers for iOS and Android remain TODO.

Until items (1)–(3) land, complex projects such as Pokémon Essentials will keep
tripping over missing constants/methods even though rendering already displays
maps/tilesets correctly.

---

## Requirements

| Tool | Notes |
|------|-------|
| Rust 1.75+ | Install via [rustup](https://rustup.rs/). |
| Ruby 3.2 (MRI) | Required by `rb-sys`. Install via rvm/rbenv/Homebrew and expose via env vars. |
| Ruby dev headers/libs | Ensure `libruby.3.2` and headers are discoverable (e.g. `/opt/homebrew/opt/ruby@3.2`). |
| Platform SDKs | Metal (macOS/iOS), Vulkan/DX (Windows/Linux), Android NDK for future mobile builds. |

Environment setup example:

```bash
export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby  # adapt to your install
```

Verify `ruby -v` and `pkg-config --libs ruby-3.2` both succeed before building.

---

## Building & Running (Desktop)

```bash
git clone https://github.com/your-org/rmxp-native-player.git
cd rmxp-native-player

export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby
export RMXP_GAME_PATH=/absolute/path/to/project   # must contain Data/, Graphics/, Audio/
# optional map override
export RMXP_START_MAP=2

cargo run -p desktop-runner
```

Environment variables:

| Variable | Description |
|----------|-------------|
| `RMXP_GAME_PATH` | Required. RMXP project root (expects `Data/System.rxdata`). |
| `RMXP_START_MAP` | Optional override for the starting map ID. |
| `RMXP_LOG` | Tracing filter (e.g. `RMXP_LOG=debug`). |

The desktop runner opens a 640×480 window, loads the configured project, and
boots Ruby. When Ruby cannot drive a Scene yet, the native tile scene remains on
screen to verify assets.

---

## Development Tips

| Task | Command |
|------|---------|
| Format everything | `cargo fmt` |
| Clippy (workspace) | `cargo clippy --all-targets --all-features` |
| Desktop smoke test | `RMXP_GAME_PATH=… cargo run -p desktop-runner` |
| RGSS bindings tests | `cargo test -p rgss-bindings --lib` |
| Faster iteration | `cargo check` |

Additional notes:

- Use `RMXP_START_MAP=<id>` to jump directly to interesting maps while debugging.
- `RMXP_LOG=trace` prints detailed Marshal parsing and RGSS binding events.
- `Graphics.snap_to_bitmap` already captures the backbuffer, enabling vanilla
  screenshot scripts during testing.

---

## RGSS Parity Roadmap

1. **Static Data Classes** – Port `Color`, `Tone`, `Rect`, `Table`, and `Font`
   to the new `StaticDataType` helpers and delete the Ruby fallback code.
2. **Resource Classes** – Rebuild `Bitmap`, `Viewport`, `Sprite`, `Plane`,
   `Window`, and `Tilemap` entirely in Rust handle tables, mirroring mkxp-z’s
   lifecycle semantics (disposed?, GC hooks, etc.).
3. **Globals & Input** – Finish `System.*` helpers, Input constants, and Hangup
   integration required by Pokémon Essentials and other projects.
4. **Behavior Audit + Tests** – Method-by-method comparisons against mkxp-z
   (blur/sharpen, window cursors, tone clamps, tilemap priorities) plus snapshot
   tests to prevent regressions.
5. **Audio / Interpreter / Persistence** – Once RGSS surfaces are 1:1, move on
   to Audio playback, scene/interpreter flow, save/config handling, and mobile
   shells.
6. **Documentation Refresh** – Keep this README and `PROGRESS.md` tracking the
   parity matrix so contributors know exactly what remains.

Contributions are welcome—please note your platform, Ruby toolchain, and sample
project when filing issues/PRs so we can reproduce behavior quickly.
