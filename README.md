# RMXP Native Player

A Rust reimplementation of the RPG Maker XP runtime. Embeds MRI Ruby 3.2 via
`rb-sys`, exposes the full RGSS 1 API surface as native typed-data classes and
module functions, and drives rendering/audio with modern Rust crates — no SDL,
no original DLLs, no Wine.

**Current status:** Pokémon Essentials 21.1 boots to the splash screen with all
402 scripts loaded and rxdata files deserialised. Vanilla RMXP projects run
their full script chain without modification.

---

## Architecture

| Crate | Role |
|-------|------|
| `apps/desktop-runner` | macOS/Windows/Linux binary: wires all crates, opens a winit window, runs the event loop. |
| `engine-core` | Boot flow, project discovery, fixed-step scheduler, platform lifecycle hooks. |
| `rgss-bindings` | Embeds Ruby MRI 3.2; native RGSS classes; preload chain; `Scripts.rxdata` evaluation. |
| `render` | wgpu/Metal renderer: tilemaps, sprites, planes, windows, screen effects. |
| `audio` | Rodio/CPAL audio — BGM/BGS/ME/SE playback, fades, memorize/restore. |
| `rmxp-data` | Marshal 4.8 reader and typed RMXP structs (`System`, `MapInfos`, maps, tilesets). |
| `platform` | Config/save directories, `tracing` logging, CLI/env helpers. |

---

## What Works

### Boot & Scripting
- Loads any RMXP project from `RMXP_GAME_PATH`
- Evaluates all `Scripts.rxdata` sections inside embedded MRI 3.2
- Full preload chain runs before user scripts: `primitives.rb` → `classic.rb` → `module_rpg1.rb` → `mkxp_wrap.rb` → `win32.rb`
- `$RGSS_SCRIPTS` global populated with `[id, name, ""]` tuples
- `rgss_main`/`rgss_stop` handled natively via Rust-driven Fiber loop
- `raise Reset` / F12 re-evaluates the entire script list in place
- `Hangup` exception raised on window close — scripts can intercept shutdown

### Data I/O
- `load_data` / `save_data` use `RGSS::Native.marshal_load` — a Rust function
  that reads the file and calls `rb_marshal_load` directly, bypassing any
  Ruby-level `Marshal.load` method corruption caused by protection scripts
- All RGSS value classes have native `_dump`/`_load` for Marshal round-trips:
  `Table` (mkxp-z binary format), `Color`/`Tone`/`Rect` (4×f64 LE, 32 bytes)
- `data_exist?`, `save_data`, and project-relative path resolution work correctly

### Graphics
- 640×480 (or resized) pixel-perfect output via wgpu/Metal
- Tilemap: map data, autotile animation, layer priorities, ox/oy scroll, tone/color tinting, flash
- Sprites: x/y/z/zoom/angle/mirror/opacity/bush_depth/blend_type, tone/color overlay, flash
- Viewports: rect clipping, ox/oy, tone/color, z-ordering
- Planes: bitmap tiling, ox/oy scroll, zoom, opacity, tone/color
- Windows: windowskin, background, contents, cursor rect, ox/oy, tone/color, openness, pause cursor
- `Graphics.freeze` / `Graphics.transition` / `Graphics.snap_to_bitmap`
- `Graphics.blur`, `Graphics.sharpen`, screen tone/brightness/flash
- `Graphics.frame_rate=` throttles the winit loop to the requested FPS
- `Bitmap`: `blt`, `stretch_blt`, `fill_rect`, `gradient_fill_rect`, `clear`, `get_pixel`, `set_pixel`, `draw_text` (built-in 8×8 font), `hue_change`

### Audio
- BGM / BGS / ME / SE: play, stop, fade via Rodio
- ME auto-resumes the interrupted BGM when playback ends
- `Audio.bgm_memorize` / `Audio.bgm_restore` hook points

### Input
- Keyboard: all RGSS buttons plus arrows, WASD, F5–F9, modifier keys
- `press?` / `trigger?` / `repeat?` / `release?` / `count` / `time?` / `dir4` / `dir8`
- Mouse: position, scroll, all five buttons
- `Input.raw_key_states` — live 256-element bool array (SDL/USB-HID scancodes)
- Clipboard and text input events
- Controller namespace returns safe defaults (no crash)

### System / Compatibility
- `System.platform`, CPU/memory stats, CSV parsing, file probes, launcher helpers
- `Win32API` shim covers `GetKeyState`, `GetAsyncKeyState`, `GetKeyboardState`, `ShowCursor`, `GetCursorPos`, `GetClientRect`, `ScreenToClient`, `FindWindowA`, `Keybd_event`
- Ruby 1.x shims: `Hash#index`, `Object#id`/`type`, `TRUE`/`FALSE`/`NIL` constants
- Full `RPG` module (RGSS 1): `RPG::Cache`, `RPG::Sprite`, `RPG::Map`, `RPG::Tileset`, `RPG::Animation`, `RPG::CommonEvent`, etc.
- `MKXP.*` compatibility aliases

---

## Known Gaps

| Item | Notes |
|------|-------|
| `Font` default propagation | `Font.default_name/size/bold/italic` exist but `Bitmap#draw_text` always uses the built-in 8×8 raster font — no TTF rendering yet. |
| Window open/close tweening | `openness` is applied instantly; should ease over frames. |
| MIDI playback | Most games use OGG/WAV; `.mid` BGM tracks need `rustysynth` integration. |
| Save-slot abstraction | `save_data`/`load_data` work for `.rxdata` files; no slot-directory or document-picker yet. |
| Mobile shells | iOS/Android launchers staged; blocked on desktop stability. |
| Controller input | Namespace stubs present; no real gamepad events yet. |
| `transition` curve | Not verified against mkxp-z's exact easing table. |

---

## Requirements

| Tool | Notes |
|------|-------|
| Rust 1.75+ | Install via [rustup](https://rustup.rs/). |
| Ruby 3.2 (MRI) | Required by `rb-sys`. Install via rvm/rbenv/Homebrew. |
| Ruby dev headers | Ensure `libruby.3.2` and headers are discoverable. |
| Platform SDKs | Metal (macOS), Vulkan/DX12 (Windows/Linux). |

```bash
export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby   # adapt to your install
```

Verify both succeed before building:
```bash
ruby -v
pkg-config --libs ruby-3.2
```

---

## Building & Running

```bash
git clone <repo>
cd rmxp-native-player

export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby
export RMXP_GAME_PATH=/absolute/path/to/project   # must contain Data/, Graphics/

cargo run -p desktop-runner
```

To run Pokémon Essentials:
```bash
export RMXP_GAME_PATH=/path/to/essentials-template
cargo run -p desktop-runner
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RMXP_GAME_PATH` | **Required.** RMXP project root (expects `Data/System.rxdata`). |
| `RMXP_START_MAP` | Optional override for the starting map ID. |
| `RMXP_LOG` | Tracing filter, e.g. `debug` or `trace`. |
| `RUST_BACKTRACE` | Set to `1` for Rust panic backtraces. |

---

## Development

| Task | Command |
|------|---------|
| Format | `cargo fmt` |
| Lint | `cargo clippy --all-targets --all-features` |
| Smoke test | `RMXP_GAME_PATH=… cargo run -p desktop-runner` |
| Bindings tests | `cargo test -p rgss-bindings --lib` |
| Quick check | `cargo check` |

- `RMXP_START_MAP=<id>` jumps directly to a specific map for renderer testing.
- `RMXP_LOG=trace` prints Marshal parsing, RGSS binding events, and frame timing.
- See `docs/rgss-parity-tracker.md` for a method-by-method coverage matrix.
