# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release   # Release build (output: target/release/taupe)
cargo run               # Run in debug mode
cargo clippy            # Lint
cargo fmt -- --check    # Check formatting
```

There are no tests currently in the project.

## Architecture

Taupe is a lossless music player built with Rust (edition 2024), using **iced** for the GUI and **rodio** for audio playback.

### Two-Thread Design

The app runs two threads that communicate via `mpsc` channels:

- **Audio thread** (`src/audio.rs`): Owns the rodio `OutputStream`/`Sink`. Receives `AudioCommand` from the GUI and sends back `AudioStatus`. Polls playback position every 250ms and detects natural playback end.
- **GUI thread** (`src/gui/app.rs`): Follows iced's Elm architecture (`App` state, `Message` enum, `update()`, `view()`). A 100ms `Tick` subscription drains pending `AudioStatus` messages from the channel.

### Key Types

```rust
// Commands sent GUI → Audio
enum AudioCommand { Load(PathBuf), Play, Pause, Stop, Seek(f32 /*ms*/), SetVolume(f32) }

// Status sent Audio → GUI
enum AudioStatus { Position(f32 /*sec*/), Duration(f32), PlaybackEnded, Metadata { .. } }
```

### Seek Interaction Pattern

The seek slider uses a two-phase approach: `SeekMoved(f32)` updates the visual position only (stored in `seek_position`), while `SeekReleased` sends the actual `AudioCommand::Seek` with milliseconds. This prevents stuttering while dragging.

### Entry Point

`src/main.rs` creates the channels, spawns the audio thread via `spawn_audio_thread()`, and launches the iced application with the custom dark theme color palette.

## Dependencies

- `iced 0.14` — GUI framework (features: tokio, svg, image)
- `rodio 0.21` — Audio playback (feature: symphonia-all for broad codec support: FLAC, ALAC, WAV, OGG, MP3, AAC, M4A, AIFF, CAF)
- `lofty 0.23` — Metadata/tag reading
- `rfd 0.17` — Native file dialogs
- `image 0.25` — Album art resizing (Lanczos3, target 80×80)

## Color Palette

```
BG:      #36302B  BG_ALT:   #635C55
TEXT:    #D9D9D9  TEXT_ALT: #AEA198
GREEN:   #9ACC9C  YELLOW:   #DAD986  RED: #D89595
```
Reference: https://coolors.co/36302b-635c55-aea198-d9d9d9-9acc9c-dad986-d89595
