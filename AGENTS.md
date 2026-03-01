# AGENTS.md

## Project Overview

Taupe is a lossless music player written in Rust (edition 2024). It uses **iced** for the GUI, **rodio** (with symphonia-all) for audio playback, **lofty** for reading audio metadata/tags, **rfd** for native file dialogs, and the **image** crate for album art processing. The project is early-stage (v0.1.2).

## Build & Run

```
cargo build --release        # release binary at target/release/taupe
cargo build                  # debug build
cargo run                    # run in debug mode
cargo clippy                 # lint
cargo fmt -- --check         # check formatting
```

There are no tests in the project currently.

## Architecture

The application has two main modules (`src/audio.rs` and `src/gui/`) that communicate over `std::sync::mpsc` channels.

### Audio thread (`src/audio.rs`)

- `spawn_audio_thread()` starts a dedicated OS thread that owns the rodio `Sink` and `OutputStream`.
- The GUI sends `AudioCommand` variants (Load, Play, Pause, Stop, Seek, SetVolume) to the audio thread.
- The audio thread sends `AudioStatus` variants (Position, Duration, Metadata) back to the GUI.
- On `Load`, it reads metadata via lofty (tags, album art, audio properties) and appends a rodio decoder to the sink.
- Position is reported back every 250ms while playing.

### GUI (`src/gui/`)

- Follows iced's Elm architecture: `App` (state), `Message` (enum), `update` (logic), `view` (layout).
- `mod.rs` re-exports and wraps `update` to return `iced::Task::none()`.
- `app.rs` contains all state, message handling, and view rendering.
- A 100ms `iced::time::every` subscription fires `Message::Tick` to drain audio status updates from the channel.
- The seek slider uses a two-phase pattern: `SeekMoved` updates visuals only; `SeekReleased` sends the actual seek command (converted to milliseconds).

### Entry point (`src/main.rs`)

- Creates the mpsc channels, spawns the audio thread, and launches the iced application.
- Defines the custom dark color palette (`custom_palette()`) and theme function.

### Color palette

Defined in both `main.rs` (as an iced `Palette`) and `app.rs` (as `const Color` values). Reference: `https://coolors.co/36302b-635c55-aea198-d9d9d9-9acc9c-dad986-d89595`

- BG: `rgb(0.212, 0.188, 0.169)` — BG_ALT: `rgb(0.388, 0.361, 0.333)`
- TEXT: `rgb(0.851, 0.851, 0.851)` — TEXT_ALT: `rgb(0.682, 0.631, 0.596)`

### Assets

SVG icons in `assets/icons/` are used for player controls (play, pause, stop). Brand assets in `assets/brand/`.

## Key Dependencies

- `iced 0.14` — GUI framework (features: tokio, svg, image)
- `rodio 0.21` — audio playback (feature: symphonia-all for broad codec support)
- `lofty 0.23` — audio metadata/tag reading
- `rfd 0.17` — native file dialog
- `image 0.25` — image decoding/resizing for album art
