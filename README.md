# RMXP Game Player (Rust + Tauri + PixiJS)

A modern, open-source reimplementation of the RPG Maker XP (RMXP) engine, built with **Tauri v2** (Rust backend) and **PixiJS** (high-performance WebGL renderer).

This project aims to allow RMXP games (like *Pokémon Essentials*) to be played on modern operating systems without relying on the original Ruby interpreter (RGSS). Instead, game logic is being ported to Rust/TypeScript, and assets are loaded natively.

## 🚀 Features

- **No Ruby Dependency**: Custom Rust-based parser (`marshal`) for reading compiled Ruby `.rxdata` files directly.
- **Cross-Platform**: Windows, macOS, and Linux support via Tauri.
- **High Performance**: 
  - **Rust Backend**: Handles file I/O, audio decoding, and heavy computation.
  - **PixiJS Frontend**: Hardware-accelerated 2D rendering for maps and sprites.
- **Audio Engine**: 
  - Supports standard formats (OGG, MP3, WAV) via `rodio`.
  - **Native MIDI Support**: Integrated `rustysynth` SoundFont synthesizer to play RMXP's default `.mid` BGM files without external drivers.

## 🛠 Tech Stack

- **Backend**: Rust (Tauri, Rodio, Rustysynth, Midly)
- **Frontend**: React 19, TypeScript, PixiJS v8
- **Build Tool**: Vite

## 📦 Installation & Setup

### Prerequisites
- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://www.rust-lang.org/) (latest stable)

### Getting Started

1.  **Install dependencies**:
    ```bash
    npm install
    ```

2.  **Run in Development Mode**:
    ```bash
    npm run tauri dev
    ```
    This will compile the Rust backend and launch the application window with Hot Module Replacement (HMR) for the frontend.

## 📂 Project Structure

- `src-tauri/src/marshal/`: Custom implementation of Ruby's `Marshal` format parser.
- `src-tauri/src/commands/`: Tauri commands for Audio, File System, etc.
- `src/`: Frontend React application.
- `src/utils/`: Helper classes (e.g., `Table` for 3D arrays).

## ⚠️ Current Status

This is a **Work In Progress**. 
- ✅ Data Loading (System, Actors, etc.)
- ✅ Audio Playback (BGM/SE/MIDI)
- 🚧 Map Rendering
- 🚧 Event System
- 🚧 Player Movement

## 📄 License

MIT
