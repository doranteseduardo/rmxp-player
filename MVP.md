# MVP Plan

**Goal:** A vanilla RMXP game boots, the title screen renders, and the player can walk around the first map.

---

## Diagnosis Summary

The foundation is solid — no stub hell, no fake completeness. Every function that exists does real work.
The gaps are specific, bounded, and additive. No architectural changes required.

### Original blockers (all resolved 2026-03-28)

| Gap | Where | Status |
|-----|-------|--------|
| Reset exception not handled | `runtime.rs` | ✅ done |
| Bundled RGSS modules missing | `scripts.rs` boot chain | ✅ done |
| Win32API shim missing | `ruby/preload/win32.rb` | ✅ done |
| `$RGSS_SCRIPTS` global not set | `lib.rs` | ✅ done |
| Ruby classic shims missing | `ruby/preload/classic.rb` | ✅ done |
| Input.count / repeat_time missing | `input.rs` | ✅ was already implemented |
| ME auto-resume BGM not wired | `audio/src/lib.rs` | ✅ done |

### What is NOT a problem

- **Game_Interpreter** — pure Ruby in mkxp-z, pure Ruby here. Works once scripts load.
- **Marshal load/save** — delegates to Ruby's own `Marshal`. Correct.
- **RGSSAD encryption** — virtually no vanilla games use it.
- **Pitch shifting** — mkxp-z doesn't implement it either. Warning log is correct.
- **Scene_Map / Game_Map / Game_Player** — pure Ruby. Not ours to implement.

---

## ✅ Phase 1 — Script Execution Correctness (done)

### 1a. `$RGSS_SCRIPTS` global
`run_scripts()` in `lib.rs` builds a Ruby `[[id, name, ""], ...]` array via
`rb_sys` and assigns it to `$RGSS_SCRIPTS` before any section is evaluated.

### 1b. Reset exception loop
`runtime.rs` detects the `Reset` exception via `rb_obj_is_kind_of`, clears it,
and returns `MainResult::Reset`. `engine-core` re-evaluates the full script list
in-place; the next frame picks up the fresh Fiber. `class Reset < Exception; end`
added to `primitives.rb`.

### 1c. Ruby classic compatibility shims
`ruby/preload/classic.rb` (ported from mkxp-z CC0): `Hash#index`, `Object#id`,
`Object#type`, `TRUE`/`FALSE`/`NIL`, `BasicObject#initialize`.

---

## ✅ Phase 2 — Bundled RGSS Modules (done)

### 2a. `module_rpg1.rb` embedded
Decoded from `mkxp-z/binding/module_rpg1.rb.xxd` (1477 lines). Evaluated in
`scripts.rs` as the third preload, before any user script. Provides:
`RPG::Cache`, `RPG::Sprite`, `RPG::Map`, `RPG::Tileset`, `RPG::Animation`,
`RPG::CommonEvent`, and the full RGSS 1 data namespace.

---

## ✅ Phase 3 — Win32API Shim (done)

### 3a. `ruby/preload/win32.rb`
Full cross-platform `Win32API` class (ported from mkxp-z CC0). Implements the
`User32` subset: `GetKeyState`, `GetAsyncKeyState`, `GetKeyboardState`,
`ShowCursor`, `GetCursorPos`, `GetClientRect`, `ScreenToClient`, `FindWindowA`,
`Keybd_event` (Alt+Enter fullscreen toggle). Unknown DLL/function combinations
return 0 silently.

---

## ✅ Phase 4 — Input Hold Tracking (was already done)

`InputStore` in `input.rs` already tracked `hold_frames`/`hold_time` per button
and exposed `Input.count`/`Input.time?`. No changes needed.

---

## ✅ Phase 5 — Audio ME Auto-Resume (done)

`AudioHandle::play_me` snapshots `BgmState` (path/volume/position), stops BGM,
plays the ME via `AudioMixer::play_me_inner`, then spawns a monitor thread that
polls `me_sink.empty()` every 100 ms and calls `mixer.play_bgm(state)` to
restore BGM when the ME finishes.

---

## ✅ Phase 6 — Integration Testing

**Not yet run.** The checklist below is the expected failure sequence — work
through it top to bottom with `RMXP_LOG=debug` against a vanilla RMXP project.

```bash
export RMXP_GAME_PATH=/path/to/vanilla/game
export RMXP_LOG=debug
cargo run -p desktop-runner 2>&1 | tee run.log
```

- [ ] Crash on `RPG::Cache` → fixed (Phase 2)
- [ ] Crash on `Win32API.new` → fixed (Phase 3)
- [ ] VM dies on F12 / Reset → fixed (Phase 1b)
- [ ] Title screen renders
- [ ] Menu input sluggish on hold → was already working (Phase 4)
- [ ] BGM cuts after ME plays → fixed (Phase 5)
- [ ] Player can enter a map and walk

---

## Remaining gaps (post-implementation audit)

These were identified during the implementation pass. None block the entire boot
sequence but each will cause a crash or visible regression at specific code paths.

### Blocking (likely crashes before title screen)

| # | Gap | File | Fix |
|---|-----|------|-----|
| 1 | `System.data_directory` missing | `crates/rgss-bindings/src/system.rs` | Add getter that returns the project `Data/` path |
| 2 | `Font` defaults not propagated to `draw_text` | `crates/rgss-bindings/src/native/bitmap.rs` | Read `Font.default_*` class attrs before drawing |
| 3 | `Bitmap.new(filename)` path resolution | `crates/rgss-bindings/src/native/bitmap.rs` | Verify `load_relative` joins against project root |

### Near-blocking (crashes at common code paths)

| # | Gap | File | Fix |
|---|-----|------|-----|
| 4 | `Graphics.frame_rate=` not honoured by event loop | `crates/engine-core/src/lib.rs` | Read stored rate and throttle `AboutToWait` accordingly |
| 5 | `Input.raw_key_states` returns empty array | `crates/rgss-bindings/src/input.rs` | Build 256-element bool vec from winit scancode state |
| 6 | `Audio.bgm_memorize`/`bgm_restore` not unified with ME slot | `crates/audio/src/lib.rs` | Route explicit memorize/restore to the same `BgmState` field |

### Polish (runs but rough edges)

| # | Gap | Notes |
|---|-----|-------|
| 7 | Window open/close tweening | `openness` setter instant; should ease over frames |
| 8 | MIDI playback | `rustysynth` integration; most games use OGG/WAV |
| 9 | Save/load slot abstraction | Works for `.rxdata`; no slot dir or document picker |
| 10 | Mobile shells | Staged; requires desktop stability first |

---

## Out of scope for MVP

- Pokémon Essentials — too many non-standard extensions; target after vanilla games work
- RGSSAD encryption — not needed for vanilla games
- Controller input — keyboard-only is fine for MVP
