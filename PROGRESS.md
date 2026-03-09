# Development Progress

## ✅ Completed Logic

### Backend (Rust / Tauri)

- **Project Structure**: Initialized Tauri + React + Vite workspace.
- **Data Parsing (`src-tauri/src/marshal/`)**:
  - Implemented a custom parser for Ruby's `.rxdata` (Marshal v4.8) format.
  - Successfully parses `System.rxdata`, `Map*.rxdata`, and other core database files.
  - Converts intricate Ruby objects (like `Table`, `Color`, `Tone`) into JSON-compatible structures for the frontend.
- **Audio Engine (`src-tauri/src/commands/audio.rs`)**:
  - Built a dedicated audio thread using `rodio`.
  - Supports standard audio formats: `.ogg`, `.mp3`, `.wav`.
  - **MIDI Support**: Integrated `rustysynth` and `midly` to synthesize `.mid` files using a SoundFont (`soundfont.sf2`), enabling playback of original RMXP BGM.
- **File System Access**:
  - Configured Tauri permissions to allow reading game directories.
  - `load_data` command securely reads and parses `.rxdata` files.

### Frontend (React / TypeScript)

- **UI Skeleton**: Basic file picker to select a game folder.
- **Data Bridge**: `App.tsx` invokes backend commands to load system data.
- **Audio Control**: Successfully plays BGM upon loading a project, respecting volume settings.
- **Utilities**: Created `Table.ts` to handle RMXP's 3D array structure (x, y, z) in TypeScript.

## 🚧 In Progress

- **Map Rendering**:
  - Re-integrating **PixiJS** for high-performance 2D rendering.
  - Need to implement a `Tilemap` renderer that consumes the `Table` data and draws the correct tiles from `Graphics/Tilesets`.
- **Game Loop**:
  - Establishing a fixed timestep loop for game logic (updates) vs. rendering (draws).

## 📝 Planned Features

### Core Engine

1.  **Sprite Rendering**: Displaying events (NPCs) and the player character.
2.  **Input Handling**: Mapping keyboard inputs (Arrow keys, Z, X, Shift) to game actions.
3.  **Collision Detection**: Implementing the passability logic (tile priorities, directional passability).
4.  **Event Interpreter**: A TypeScript implementation of the event command list (Map events, Common events).

### UI / HUD

- **Window System**: Porting the `Window_Base` logic to React/PixiJS (message boxes, menus).
- **Scene Management**: Switching between Title, Map, Menu, and Battle scenes.

### Polish

- **Save/Load**: Serializing game state back to `.rxdata` or a custom JSON save format.
- **Audio Effects**: Implementing BGS (Background Sounds), ME (Music Effects), and SE (Sound Effects) with pitch/pan support.
