# Development Progress

## ✅ Bootstrapped

- Workspace migrated to a multi-crate layout (`engine-core`, `render`, `audio`,
  `platform`, `rgss-bindings`, `mobile-shell`, `desktop-runner`).
- Desktop runner crate launches a winit event loop and renders actual RMXP
  tilemaps via the shared renderer abstraction (Metal on macOS, Vulkan/DX on
  Linux/Windows).
- Audio subsystem initializes the default rodio output stream (no playback yet).
- RGSS `Audio.*` API is now implemented natively with memorize/restore semantics and hook points so the engine can drive real playback instead of Ruby stubs.
- Platform helper configures config/save directories and installs `tracing`
  logging with `RMXP_LOG` filtering.
- RGSS bindings crate boots an embedded Ruby 3.2 VM via `rb-sys`; building now
  requires a Ruby toolchain (`RB_SYS_RUBY_VERSION=3.2`, `RUBY=/path/to/ruby`).
- `RGSS::Native` bridge mirrors `Bitmap`, `Viewport`, `Sprite`, and `Window`
  classes so Ruby scripts manipulate the canonical state that Rust snapshots for
  rendering/audio subsystems.
- Plane and Tilemap RGSS classes now map to native handle stores; Ruby assigns
  tilesets/autotiles/priorities via `Table` blobs and the renderer rebuilds the
  map each frame using those live snapshots (ox/oy scroll respected).
- Kernel helpers (`load_data`, `save_data`, `data_exist?`) resolve project paths
  and reuse Ruby `Marshal`, so vanilla scripts can read/write `.rxdata` without
  touching the Rust side.
- Graphics/Bitmap built-ins now run natively: `Bitmap#blt`, `Bitmap#fill_rect`,
  `Bitmap#get_pixel`/`set_pixel`, `Bitmap#clear`, plus `Graphics.freeze`,
  `Graphics.snap_to_bitmap`, and the tone/brightness/flash pipeline manipulate
  the same textures the renderer consumes, while script-level Cache logic is
  left entirely to project `Scripts.rxdata`. Hue shifts (`Bitmap#hue_change`)
  and per-sprite flashes now route through the native renderer instead of Ruby
  shims.
- `Bitmap#stretch_blt`, `Bitmap#gradient_fill_rect`, and `Bitmap#draw_text` now
  render through a built-in 8×8 ASCII font so the standard RMXP windows and UI
  code can draw text/gradients without modification.
- Added `rmxp-data` crate with a Marshal reader/JSON bridge plus engine wiring
  that reads `Data/System.rxdata`/`MapInfos.rxdata` from `RMXP_GAME_PATH`.
- Engine now parses the start map and feeds a rendered tile scene (tileset +
  autotiles) into pixels so we can visualize `.rxdata` content end-to-end.
- Renderer handles autotile sampling/animation, multi-layer composition, and
  RGSS priority tables so ground/overlay layers display correctly.
- Desktop runner hosts a fixed 60 Hz loop with keyboard input (arrows/WASD),
  pixel-perfect camera scroll (640×480 viewport), and a placeholder player
  marker to visualize movement on real maps.
- RGSS viewports, sprites, planes, and windows now composite directly in the
  renderer (viewport clipping, sprite zoom/angle/mirror, tone/color overlays,
  plane tiling, windowskin/background/contents/cursor rendering), enabling
  vanilla `Scripts.rxdata` scenes to drive the entire frame without native
  stubs.
- Window close events no longer abort the process; instead they request a
  `Graphics` hangup that raises the RGSS `Hangup` exception on the next
  `Graphics.update`, matching mkxp-z’s lifecycle semantics.
- Kernel `rgss_main`/`rgss_stop` are handled entirely in Rust now: the VM
  captures the scene block, resumes it through `RGSS::Runtime`, and exposes a
  native interpreter command queue (`RGSS::Native.interpreter_request_*`) so Ruby
  can request pauses or map reloads without custom shims.
- The extended mkxp-z `System` APIs (platform/OS queries, CPU & memory info,
  CSV parsing, launcher helpers, default font family, file existence checks) are
  implemented, so projects that lean on those helpers no longer crash during
  boot.
- Input now mirrors mkxp-z’s helpers: `release?`, `count`, `time?`, mouse coords,
  scroll deltas, clipboard/text input, and controller namespace stubs all exist,
  with blur/tone buttons feeding the native renderer instead of Ruby shims.
- `Graphics.blur`/`Graphics.sharpen` hook into the renderer’s screen-effects
  pass (with frozen-frame fallbacks) so transitions that rely on those filters
  match the RPG Maker runtime instead of silently no-oping.
- Tilemap flash tables now flow from Ruby `flash_data` to the renderer with the
  mkxp-z pulse curve, opacity clamping, and additive blend, while window cursor
  rectangles respect `ox/oy` so scrolled contents keep their highlight and tone/
  color operations mirror the reference implementation.

## ✅ MVP Unblocking Pass (2026-03-28)

All items below were the six critical gaps between the existing foundation and
a playable vanilla RMXP title screen. Every item is now implemented and the
workspace builds cleanly with zero warnings.

- **Reset exception loop** – `runtime.rs` detects the `Reset` Ruby exception via
  `rb_obj_is_kind_of`, clears it, and returns `MainResult::Reset` up the call
  stack. `engine-core` re-evaluates the full `Scripts.rxdata` section list in
  place, then the next frame picks up the fresh Fiber. F12 / in-game resets now
  work correctly.
- **`$RGSS_SCRIPTS` global** – `run_scripts()` builds a Ruby array of
  `[id, name, ""]` tuples via `rb_sys` and assigns it to `$RGSS_SCRIPTS` before
  evaluating any section. Script-existence checks (e.g. Essentials version
  detection) no longer raise `NameError`.
- **`Reset` class** – defined in `primitives.rb` as `class Reset < Exception; end`
  so game scripts that call `raise Reset` work without a native binding.
- **Preload chain** – `scripts.rs` now evaluates five layers before any user
  script runs:
  1. `primitives.rb` — `RGSS::Runtime`, `Hangup`, `Reset`, data I/O helpers.
  2. `classic.rb` — Ruby 1.x→3.x shims (`Hash#index`, `Object#id/type`,
     `TRUE`/`FALSE`/`NIL`, `BasicObject#initialize`).
  3. `module_rpg1.rb` — full `RPG` namespace decoded from mkxp-z's
     `binding/module_rpg1.rb.xxd`: `RPG::Cache`, `RPG::Sprite`, `RPG::Map`,
     `RPG::Tileset`, `RPG::Animation`, `RPG::CommonEvent`, and the rest of the
     1477-line RGSS 1 stdlib.
  4. `mkxp_wrap.rb` — `MKXP` compatibility aliases for older mkxp API names.
  5. `win32.rb` — Cross-platform `Win32API` class. Routes `User32` calls
     (`GetKeyState`, `GetAsyncKeyState`, `GetKeyboardState`, `ShowCursor`,
     `GetCursorPos`, `GetClientRect`, `ScreenToClient`, `FindWindowA`,
     `Keybd_event` fullscreen toggle) to native equivalents. Unknown DLL/function
     combinations are tolerated silently.
- **ME auto-resume BGM** – `AudioHandle::play_me` snapshots `BgmState`
  (path/volume/position) before stopping BGM, then spawns a monitor thread that
  polls `me_sink.empty()` every 100 ms and calls `mixer.play_bgm(state)` once
  the ME finishes, matching mkxp-z's auto-restore semantics.
- **`eval_preload` helper** – `RubyVm::eval_preload(code, label)` wraps
  `eval` with a `ScriptLabelGuard` so preload errors report their origin label
  instead of `(unknown script)`.

## ✅ MVP Blocking/Near-Blocking Gaps Resolved

All six blocking and near-blocking gaps are now closed. Only polish items remain.

## ✅ Pokémon Essentials Boot Chain (2026-03-28)

Fixed all blocking issues preventing PE 21.1 from booting. The engine now loads
all 402 PE scripts and reaches the splash screen.

- **`RGSS::Native.marshal_load(path)`** — Added a Rust-backed native function
  that reads a file with `std::fs::read` and calls `rb_marshal_load` at the C
  level, bypassing Ruby-level `Marshal.load` dispatch entirely. PE protection
  scripts remove `Marshal.load`'s singleton method (causing `Kernel#load` to be
  dispatched instead, which executes rxdata as Ruby source and returns `true`).
  The C-level call is immune to this.

- **Value class `_dump`/`_load`** — All four native value classes now serialise
  and deserialise correctly with Marshal:
  - `Table` — mkxp-z binary format: `[dim:i32][xsize:i32][ysize:i32][zsize:i32][count:i32][data:i16*]`
  - `Color`, `Tone`, `Rect` — 32-byte format: four `f64` LE values

- **`argc=-1` calling convention fix** — All 69 `rb_define_module_function`
  registrations used a positive `argc` (1, 2, 4, 5…) but the C function had the
  variadic `(c_int, *const VALUE, VALUE)` signature. Ruby's ABI for fixed-argc
  methods passes `(self, arg1, arg2…)` instead of `(argc, argv, self)`, so
  `argc` was receiving the module's VALUE (garbage) and `argv` was receiving the
  first argument's VALUE as a raw pointer — causing misaligned dereference
  panics and TypeError crashes. Fixed globally by changing every positive argc
  to `-1`.

- **Sprite graceful degradation** — `get_sprite` now returns `Option` so methods
  called on Sprite subclasses that bypass the native allocator return `nil`
  instead of panicking.

## ✅ Post-Splash Stability Pass (2026-03-29)

Fixed all blocking issues preventing PE 21.1 from advancing past the splash
screen into gameplay. The engine now enters the main game loop.

- **Fiber-based game loop** — `run_scripts()` now wraps the last (Main) script
  section in a `Fiber` via `RGSS::Runtime.install_main_from_source`, instead of
  evaluating it synchronously. The event loop drives the fiber one frame at a
  time via `resume_main_loop()`. This makes both PE-style synchronous games
  (mainFunction called directly) and standard `rgss_main { }` games work
  identically: `Graphics.update` → `Fiber.yield` suspends back to the event
  loop for rendering without ever blocking it. `kernel_rgss_main` updated to
  call the block directly in the already-running fiber context (no nested fiber).

- **PE clone protection bypass** — PE's protection mechanism redefines `clone`
  on every `RPG::*` class with a stub that raises `NoMethodError`. This bypasses
  `undef_method`/`remove_method` interception and makes `method_defined?` return
  `true` while the method raises. Fixed by adding a one-shot `Graphics.update`
  hook that unconditionally redefines `clone` on all `RPG::*` classes on the
  first frame — fires after PE protection runs but before any BGM/audio code
  that clones `RPG::AudioFile`.

- **`Bitmap` subclass graceful degradation** — `get_bitmap()` now returns
  `Option<&mut BitmapValue>` instead of panicking. All callers use
  `let Some(data) = get_bitmap(...) else { return ... }`. `bitmap_disposed_q`
  returns `Qtrue` (treated as disposed) when no native data is present. Matches
  the existing pattern for `Sprite`.

- **macOS `NSScreen` enumeration panic** — `engine_center_window()` and all
  winit monitor-enumeration paths trigger an `icrate-0.0.4` `NSUInteger` vs
  `NSInteger` type-code mismatch panic on macOS. Window centering is cosmetic;
  `engine_center_window()` is now a no-op until `icrate` is updated or replaced
  with a CoreGraphics path.

## 🚧 Polish (game runs, rough edges)

These are the known gaps between the current state and a fully playable vanilla
RMXP game (title screen + first map walkable). None require architectural
changes — all are additive.

### Resolved

- ✅ `Graphics.frame_rate=` throttles the winit loop via `current_frame_rate()` sleep.
- ✅ `Input.raw_key_states` returns a live 256-element SDL/USB-HID scancode array.
- ✅ All value classes (`Table`, `Color`, `Tone`, `Rect`) have native `_dump`/`_load`.
- ✅ PE 21.1 boots to the splash screen with all 402 scripts loaded.

### Remaining

1. **`Font` rendering** – `Font.default_name/size/bold/italic` class defaults exist
   but `Bitmap#draw_text` always uses the built-in 8×8 raster font. TTF rendering
   not yet integrated.

2. **`Audio.bgm_memorize`/`bgm_restore` state** – Ruby-side hooks are bound; the
   memorize slot is not yet shared with the ME auto-resume slot, so explicit calls
   don't persist across ME playback.

3. **Window open/close tweening** – `openness` is applied instantly; should ease
   over several frames matching mkxp-z's animation curve.

4. **MIDI playback** – `rustysynth` integration for `.mid` BGM. Most games use
   OGG/WAV; only affects titles that ship MIDI tracks.

5. **Save-slot abstraction** – `save_data`/`load_data` work for `.rxdata` files;
   no save-slot directory or document-picker for iOS/Android.

6. **Mobile shells** – iOS/Android launchers staged; blocked on desktop stability.

7. **Controller input** – Namespace stubs present; no real gamepad events yet.
