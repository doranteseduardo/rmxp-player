<div align="center">

# RMXP Native Player

**A Rust reimplementation of the RPG Maker XP runtime**

[![Rust](https://img.shields.io/badge/Rust-stable-CE422B?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Ruby](https://img.shields.io/badge/Ruby-MRI%203.2-CC342D?style=flat-square&logo=ruby)](https://www.ruby-lang.org/)
[![wgpu](https://img.shields.io/badge/renderer-wgpu%2FMetal-FF6600?style=flat-square)](https://wgpu.rs/)
[![License](https://img.shields.io/badge/License-MIT-10B981?style=flat-square)](LICENSE)

Embeds MRI Ruby 3.2 via `rb-sys`, exposes the full RGSS 1 API surface as native typed-data classes and module functions, and drives rendering/audio with modern Rust crates — no SDL, no original DLLs, no Wine. Pokémon Essentials 21.1 boots past the splash screen into the main game loop; all 402 scripts load cleanly.

</div>

---

## What it does

| Subsystem | Description |
|---|---|
| **Boot & Scripting** | Loads any RMXP project, evaluates `Scripts.rxdata` inside embedded MRI 3.2, drives main Fiber at 60 Hz |
| **Graphics** | wgpu/Metal renderer — tilemaps, sprites, planes, windows, viewports, screen effects, transitions |
| **Audio** | Rodio/CPAL — BGM/BGS/ME/SE playback, fades, memorize/restore; ME auto-resumes interrupted BGM |
| **Input** | Keyboard (all RGSS buttons + WASD), mouse (5 buttons, scroll), `Input.raw_key_states` |
| **Data I/O** | `load_data`/`save_data` via native Rust Marshal; `Table`, `Color`, `Tone`, `Rect` round-trips |
| **Compatibility** | `Win32API` shim, Ruby 1.x shims, full `RPG` module, `MKXP.*` aliases |

---

## Stack

| Crate | Role |
|---|---|
| `apps/desktop-runner` | macOS/Windows/Linux binary — wires all crates, opens winit window, runs event loop |
| `engine-core` | Boot flow, project discovery, fixed-step scheduler, platform lifecycle hooks |
| `rgss-bindings` | Embeds Ruby MRI 3.2; native RGSS classes; preload chain; `Scripts.rxdata` evaluation |
| `render` | wgpu/Metal renderer: tilemaps, sprites, planes, windows, screen effects |
| `audio` | Rodio/CPAL audio — BGM/BGS/ME/SE playback, fades |
| `rmxp-data` | Marshal 4.8 reader and typed RMXP structs |
| `platform` | Config/save directories, `tracing` logging, CLI/env helpers |

---

## Getting started

### Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- Ruby MRI 3.2 with dev headers (`libruby.3.2`)
- Metal (macOS) or Vulkan/DX12 (Windows/Linux)

```bash
export RB_SYS_RUBY_VERSION=3.2
export RUBY=$HOME/.rvm/rubies/ruby-3.2.0/bin/ruby
export RMXP_GAME_PATH=/absolute/path/to/project

cargo run -p desktop-runner
```

### Environment variables

| Variable | Description |
|---|---|
| `RMXP_GAME_PATH` | Required. RMXP project root (expects `Data/System.rxdata`) |
| `RMXP_START_MAP` | Optional map ID override for renderer testing |
| `RMXP_LOG` | Tracing filter, e.g. `debug` or `trace` |

---

## Architecture

```
rmxp-native-player/
├── apps/
│   └── desktop-runner/      # Binary entrypoint — winit window + event loop
├── engine-core/             # Boot flow, fixed-step scheduler
├── rgss-bindings/           # Ruby MRI embed, RGSS native classes
│   └── preload/
│       ├── primitives.rb    # Core RGSS primitives
│       ├── classic.rb       # Classic RMXP helpers
│       ├── module_rpg1.rb   # RPG module (RGSS 1)
│       ├── mkxp_wrap.rb     # mkxp-z compatibility
│       └── win32.rb         # Win32API shim
├── render/                  # wgpu/Metal renderer
├── audio/                   # Rodio audio engine
├── rmxp-data/               # Marshal 4.8 parser + RMXP types
└── platform/                # Config, logging, CLI helpers
```

---

## Roadmap

```
[x] Ruby Marshal v4.8 parser/serializer
[x] Scripts.rxdata evaluation inside embedded MRI 3.2
[x] wgpu/Metal renderer — tilemaps, sprites, planes, windows
[x] Fiber-based event loop at 60 Hz
[x] Audio — BGM/BGS/ME/SE with Rodio
[x] Full keyboard + mouse input
[x] Win32API shim
[x] RPG module (RGSS 1) + MKXP compatibility
[x] Pokémon Essentials 21.1 boots (402 scripts)
[ ] TTF font rendering (Bitmap#draw_text)
[ ] Window openness easing
[ ] MIDI playback (rustysynth)
[ ] Gamepad input
[ ] iOS / Android launchers
```

---

<div align="center">
  <sub>Not affiliated with Enterbrain, Kadokawa, or the Pokémon Essentials team · MIT License</sub>
</div>
