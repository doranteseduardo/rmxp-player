# RGSS Parity Tracker

Tracks every mkxp-z surface against this project's implementation.
Sources: `mkxp-z/binding/*.cpp`, `mkxp-z/binding/module_rpg*.rb.xxd`,
`mkxp-z/scripts/preload/`.

Legend: ✅ complete · ⚠️ partial · ❌ missing

---

## Core Modules

| Area | Status | Notes |
|------|--------|-------|
| `Graphics` module | ⚠️ partial | All documented methods bound. `frame_rate=` now throttles the winit event loop via `current_frame_rate()` sleep. `transition` curve not verified against mkxp-z. |
| `Audio` module | ⚠️ partial | BGM/BGS/ME/SE play/stop/fade all wired to rodio. ME auto-resume BGM on completion implemented. `bgm_memorize`/`bgm_restore` Ruby hooks exist but don't share state with ME auto-resume slot. Pitch shifting logged as unsupported. MIDI not integrated. |
| `Input` module | ⚠️ partial | `press?`/`trigger?`/`repeat?`/`release?`/`count`/`time?`/`dir4`/`dir8` complete. Mouse x/y/scroll, clipboard, text input complete. `raw_key_states` returns a live 256-element bool array (SDL/USB-HID indexed) from winit. Controller namespace stubbed. |
| `System` module | ⚠️ partial | Platform detection, CPU/memory, user/language, CSV parsing, file_exist?, launcher helpers implemented. `data_directory` missing (gap #1). JSON settings persistence missing. |
| `Kernel` (`rgss_main`, `rgss_stop`, `load_data`, `save_data`) | ✅ complete | All wired natively. `$RGSS_SCRIPTS` global set before user scripts. Reset exception detected and re-runs full script list. |

---

## Resource Classes

| Class | Status | Notes |
|-------|--------|-------|
| `Bitmap` | ⚠️ partial | `blt`, `stretch_blt`, `fill_rect`, `gradient_fill_rect`, `clear`, `get_pixel`, `set_pixel`, `draw_text`, `hue_change` all implemented. Path resolution for `Bitmap.new(filename)` needs verification against project root (gap #3). Font defaults not honoured (gap #2). |
| `Sprite` | ⚠️ partial | All properties implemented. `wave_amp`/`wave_length` not implemented (rare). Flash state implemented. `bush_depth` wired. |
| `Viewport` | ⚠️ partial | rect/ox/oy/tone/color/z/visible all wired. Nested viewport z-ordering not tested. |
| `Plane` | ⚠️ partial | scroll/zoom/opacity/tone/color all wired. Blend mode parity with mkxp-z not verified. |
| `Window` | ⚠️ partial | All properties wired. `openness` setter exists but animation is instant (gap #7). Pause animation wired. `open`/`close` tweening not implemented. |
| `Tilemap` | ⚠️ partial | map_data, priorities, autotiles, flash, ox/oy, tone/color all wired. Autotile animation runs. VX/VX Ace tile format variants not needed for RGSS 1. |

---

## Value Classes

| Class | Status | Notes |
|-------|--------|-------|
| `Color` | ✅ complete | Native typed-data. `_dump`/`_load` (32 bytes: 4×f64 LE) implemented. |
| `Tone` | ✅ complete | Native typed-data. `_dump`/`_load` (32 bytes: 4×f64 LE) implemented. |
| `Rect` | ✅ complete | Native typed-data. `_dump`/`_load` (32 bytes: 4×f64 LE, cast to i32) implemented. |
| `Table` | ✅ complete | Native typed-data. `_dump`/`_load` (mkxp-z binary format: 5×i32 header + i16 array) implemented. |
| `Font` | ⚠️ partial | `default_name`/`default_size`/`default_bold`/`default_italic`/`default_color` class methods present but `draw_text` falls back to built-in 8×8 font regardless of name (gap #2). |

---

## Ruby Stdlib / Preloads

| Component | Status | Notes |
|-----------|--------|-------|
| `RGSS::Runtime` (Fiber loop) | ✅ complete | install_main, resume_main, yield_frame, active?, reset, notify_suspend/resume/low_memory. |
| `Hangup` exception | ✅ complete | Raised on window close; matches mkxp-z lifecycle. |
| `Reset` exception | ✅ complete | Defined in primitives.rb; detected natively in runtime.rs; triggers full script re-evaluation. |
| Ruby classic shims | ✅ complete | `Hash#index`, `Object#id`/`type`, `TRUE`/`FALSE`/`NIL`, `BasicObject#initialize`. |
| `RPG` module (RGSS 1) | ✅ complete | Full `module_rpg1.rb` decoded from mkxp-z and loaded before user scripts. Provides `RPG::Cache`, `RPG::Sprite`, `RPG::Map`, `RPG::Tileset`, `RPG::Animation`, `RPG::CommonEvent`, etc. |
| `MKXP` compat aliases | ✅ complete | `MKXP.data_directory`, `MKXP.raw_key_states`, `MKXP.mouse_in_window`. |
| `Win32API` shim | ✅ complete | Cross-platform `Win32API` class. Implements `User32`: `GetKeyState`, `GetAsyncKeyState`, `GetKeyboardState`, `ShowCursor`, `GetCursorPos`, `GetClientRect`, `ScreenToClient`, `FindWindowA`, `Keybd_event`. Unknown calls tolerated silently. |
| `fileutils` | ⚠️ partial | Minimal `mkdir_p` polyfill in primitives.rb. Full `require 'fileutils'` falls through to MRI stdlib (available if Ruby 3.2 is on PATH). |
| `zlib` / `yaml` / `json` | ✅ via MRI | Available through the system Ruby 3.2 stdlib; no custom vendoring needed. |
| `Win32API` (native Windows) | ❌ not needed | We don't embed native Win32API; the shim covers all practical cases. |
| RGSS 2 / RGSS 3 modules | ❌ out of scope | `module_rpg2/3.rb` for VX/VX Ace — not an RMXP target. |

---

## Infrastructure

| Component | Status | Notes |
|-----------|--------|-------|
| `$RGSS_SCRIPTS` global | ✅ complete | Set before any user script runs. Array of `[id, name, ""]`. |
| Reset loop | ✅ complete | Full script re-evaluation on `raise Reset`. |
| ME → BGM auto-resume | ✅ complete | Monitor thread restores memorized BGM when ME sink empties. |
| Audio BGM memorize/restore (explicit) | ⚠️ partial | Ruby hooks bound; shared state slot with ME auto-resume needs unification. |
| Save/load slots | ❌ missing | `save_data`/`load_data` work for `.rxdata` but no slot-abstraction or document picker. |
| RGSSAD encryption | ❌ not implemented | Virtually no vanilla games use it; low priority. |
| Mobile shells (iOS/Android) | ❌ staged | Awaiting desktop stability. |
| MIDI audio | ❌ not implemented | `rustysynth` integration planned. Most games use OGG/WAV. |
| Controller input | ❌ stubbed | `Controller.*` namespace returns safe defaults. Keyboard-only for MVP. |

---

## Known Gaps to Close Next

Pokémon Essentials 21.1 boot chain reaches the splash screen. Remaining items:

1. Window open/close tweening — `openness` setter is instant; should ease over frames.
2. MIDI playback — `rustysynth` integration for `.mid` BGM tracks.
3. Save/load slot abstraction — no slot directory or document-picker integration.
4. Mobile shells — iOS/Android launchers staged; blocked on desktop stability.
5. `transition` curve — not verified against mkxp-z's exact easing.
6. PE `BitmapWrapper#animated?` — method missing in essentials-template; PE-internal Ruby issue.
