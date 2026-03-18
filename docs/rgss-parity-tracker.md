# RGSS Parity Tracker

This checklist captures every mkxp-z surface that Pokémon Essentials expects so
we can drive it to “implemented/verified” and stop discovering missing methods at
runtime. Sources are culled from `mkxp-z/binding/*.cpp` plus the bundled Ruby
stdlib (`module_rpg*.rb`, `fileutils`, `enc/` etc.).

| Area | mkxp-z Source | Status in this repo | Notes / Gaps to Close |
|------|---------------|---------------------|-----------------------|
| Graphics module | `graphics-binding.cpp` | **Partial** – most setters/getters exist, but resize/center/fullscreen/window metrics still need mkxp-z semantics & tests; `Graphics.transition`, blur/sharpen caches, and FPS reporting require verification. | Finish method-by-method audit + snapshot tests. |
| Bitmap | `bitmap-binding.cpp` | **Mostly done** – drawing/fill/text/hue implemented; font ownership + `Bitmap.new(filename)` error handling, `clear_rect`, and `blt` edge cases still need comparison. | Mirror mkxp-z’s error paths and GC hooks. |
| Viewport | `viewport-binding.cpp` | **Partial** – viewport rect/ox/oy/tone/color exist but haven’t been cross-checked vs mkxp-z (esp. nested viewport z ordering). | Add parity tests. |
| Sprite | `sprite-binding.cpp` | **Partial** – base fields wired; still missing bush depth semantics, `wave_amp/length`, `flash(color,duration)` behavior, and disposed? lifecycle tests. | Port remaining setters. |
| Plane | `plane-binding.cpp` | **Partial** – scrolling/zoom implemented but `z` stacking, tone/color combos, and bitmap GC parity need checks. |
| Window | `window-binding.cpp` | **Partial** – tone/color/cursor fixes landed today, but `windowskin` contents caching, pause animation, `open/close` tweening, and `active` semantics remain TODO. |
| Tilemap | `tilemap-binding.cpp` | **Partial** – map/priorities/flash/tone hooked up; need proper `update` animation for autotiles, VX variants, and passability tables. |
| Table/Color/Tone/Rect/Font | `table-binding.cpp`, etc. | **Mostly done** – need to audit serialization, `Table#[]=`, and default font propagation vs mkxp-z defaults. |
| Audio | `audio-binding.cpp` | **Partial** – Ruby `Audio.*` entry points plus memorize/restore/fade semantics now live in Rust and forward to pluggable hooks, but the actual rodio backend still needs to be wired in (currently logs warnings). |
| Input | `input-binding.cpp` | **Mostly done** – release?/count/time?, mouse/touch done; controller namespace still stubbed (no actual device events). |
| System | `etc-binding.cpp` & helpers | **Partial** – platform detection/logging implemented; still missing `System.data_directory`, cache reload, launcher helpers, and CSV utilities used by Essentials. |
| Interpreter/Scene loop | `scene` classes in mkxp-z | **Missing** – we currently load scripts but never run Game_Map/Game_Player/Game_Interpreter; need to port the scene scheduler and event interpreter. |
| Persistence | mkxp-z filesystem helpers | **Missing** – no save/config slots or document picker integration yet. |
| Ruby stdlib bundle | `binding/module_rpg*.rb.xxd`, packaged stdlib | **Incomplete** – only MRI core is available; we must ship the mkxp-z stdlib subset (`fileutils`, `zlib`, `yaml`, encodings, Win32API stubs, RPG modules) so `require` calls succeed. |

Next actions:
1. Port remaining RGSS classes/modules using the mkxp-z bindings as the source of truth.
2. Vendor the mkxp-z stdlib bundle (or build tooling to copy it from the user’s Ruby install).
3. Add automated smoke tests that boot Essentials and a vanilla RMXP project headless to catch regressions quickly.
