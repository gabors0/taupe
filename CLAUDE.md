# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build --release   # Release build (output: target/release/taupe)
cargo run               # Run in debug mode
cargo clippy            # Lint
cargo fmt -- --check    # Check formatting
cargo test              # Unit tests (src/audio.rs — pick_picture_index logic)
```

### System Dependencies (Ubuntu/Debian)

Building requires system libraries for audio, GTK file dialogs, and D-Bus (media controls):

```bash
sudo apt install libasound2-dev libgtk-3-dev libdbus-1-dev pkg-config
```

## Architecture

Taupe is a lossless music player built with Rust (edition 2024), using **iced** for the GUI, **rodio** for audio playback, and **souvlaki** for OS media control integration (MPRIS on Linux, media keys on Windows/macOS).

### Project Structure

```
src/
├── main.rs              # Entry point, channels, theme, keyboard/mouse subscriptions, souvlaki setup
├── audio.rs             # Audio thread: rodio playback, metadata extraction (lofty), unit tests
└── gui/
    ├── mod.rs           # Module exports (App, Message, view, update)
    └── app.rs           # Elm architecture: App state, Message enum, update(), view()

assets/
├── brand/               # Logo images (full.png, icon_1k.png, icon_500.png)
└── icons/               # SVG playback controls (play, pause, next, prev, stop, load, folder)

.github/workflows/
└── build.yml            # CI: build, test, clippy (warnings = errors)
```

### Two-Thread Design

The app runs two threads that communicate via `mpsc` channels:

- **Audio thread** (`src/audio.rs`): Owns the rodio `OutputStream`/`Sink`. Receives `AudioCommand` from the GUI and sends back `AudioStatus`. Polls playback position every 250ms and detects natural playback end.
- **GUI thread** (`src/gui/app.rs`): Follows iced's Elm architecture (`App` state, `Message` enum, `update()`, `view()`). A 100ms `Tick` subscription drains pending `AudioStatus` messages from the channel.

### Key Types

```rust
// Commands sent GUI → Audio
enum AudioCommand { Load(PathBuf), Play, Pause, Stop, Seek(f32 /*ms*/), SetVolume(f32) }

// Status sent Audio → GUI
enum AudioStatus {
    Position(f32 /*sec*/), Duration(f32), PlaybackEnded,
    Metadata { title, artist, album, track_no, disc_no, picture: Option<(Vec<u8>, String)>,
               sample_rate_hz, bitrate_kbps, channels, bit_depth, file_format }
}

enum PlaybackState { Stopped, Playing, Paused }

// Per-track info scanned at folder load time
struct TrackInfo { index, path, title, artist, album, track_no, duration_secs }
```

### Playlist & Auto-Advance

- Loading a file or folder scans all audio files in the directory and builds `App::playlist` (sorted `Vec<PathBuf>`) + `App::tracks` (`Vec<TrackInfo>`, metadata read upfront via lofty).
- `App::playlist_index` tracks the currently playing track; `App::selected_index` tracks the highlighted row (can differ).
- On `PlaybackEnded`, the GUI auto-advances to the next track (`play_track(app, idx + 1)`) if one exists; otherwise stops.
- Double-clicking a playlist row fires `PlaylistRowDoubleClicked(idx)` → `play_track`.
- The playlist table uses `iced::widget::table` with responsive column sizing; wide layout (≥750px) shows Artist and Album columns.

### Seek Interaction Pattern

The seek slider uses a two-phase approach: `SeekMoved(f32)` updates the visual position only (stored in `seek_position`), while `SeekReleased` sends the actual `AudioCommand::Seek` with milliseconds. This prevents stuttering while dragging.

### OS Media Controls (souvlaki)

- `MediaControls` are created in `main.rs` and passed to the `App` via `Rc<RefCell<>>`.
- The controls are attached lazily — `attach()` is called on the first track load, not at startup.
- Media events (play/pause/next/prev/seek) are received via a separate `mpsc` channel and polled during `Tick`.
- Album art is written to `/tmp/taupe_cover.png` for the OS media overlay.

### Keyboard & Mouse Shortcuts

- **Space**: Toggle play/pause
- **Left/Right arrows**: Seek backward/forward 5 seconds
- **Up/Down arrows**: Volume up/down 5%
- **Mouse wheel**: Volume up/down

These are handled via iced keyboard/mouse subscriptions registered in `main.rs`.

### Entry Point

`src/main.rs` creates the channels, spawns the audio thread via `spawn_audio_thread()`, sets up souvlaki media controls, registers keyboard and mouse subscriptions, and launches the iced application with the custom dark theme color palette.

## Dependencies

- `iced 0.14` — GUI framework (features: tokio, svg, image, lazy)
- `rodio 0.21` — Audio playback (feature: symphonia-all for broad codec support: FLAC, ALAC, WAV, OGG, MP3, AAC, M4A, AIFF, CAF)
- `lofty 0.23` — Metadata/tag reading
- `rfd 0.17` — Native file dialogs
- `image 0.25` — Album art resizing (Lanczos3, target 80×80)
- `souvlaki 0.8` — OS media control integration (MPRIS/media keys)

## CI/CD

GitHub Actions workflow (`.github/workflows/build.yml`) runs on every push and PR:

1. Installs system dependencies
2. Runs `cargo build --release`
3. Runs `cargo test`
4. Runs `cargo clippy -- -D warnings` (warnings treated as errors)

## Color Palette

```
BG:      #36302B  BG_ALT:   #635C55
TEXT:    #D9D9D9  TEXT_ALT: #AEA198
GREEN:   #9ACC9C  YELLOW:   #DAD986  RED: #D89595
```
Reference: https://coolors.co/36302b-635c55-aea198-d9d9d9-9acc9c-dad986-d89595
