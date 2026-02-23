<h2 align=center>taupe</h2>
<h4 align=center>lossless music player made in rust using rodio and iced</h4>

> [!IMPORTANT]
> this project is still *very* far from done, look at the todo list to see the avaiable features

### Todo/Roadmap
<details>
  <summary>Audio formats </summary>
  
  - [x] flac
  - [x] alac
  - [x] wav
  - [x] vorbis/ogg
  - [x] mp3
  - [x] aac
  - [x] m4a/mp4
  - [x] aiff
  - [x] caf
  
</details>
<details>
  <summary>Player functions</summary>

  - [x] Load file
  - [x] Play, pause, stop
  - [x] Volume
  - [x] Seek
  - [x] Get metadata of songs
  - [ ] Playback queue
  - [ ] Shuffle
  - [ ] Repeat (one/all)
   
</details>
<details>
  <summary>UI</summary>

  - [ ] App icon/logo
  - [x] Album art display
  - [ ] Keyboard shortcuts
  - [ ] Settings menu
  - [ ] Light theme
  - [ ] Visualizers
  - [ ] Equalizer
   
</details>
<details>
  <summary>Build & Install</summary>

  - [ ] cargo install
  - [ ] AUR package
  - [ ] Windows and macOS setups
  - [ ] Flatpak
  - [ ] AppImage
   
</details>

### Building

```bash
git clone https://github.com/gabors0/taupe.git
cd taupe
cargo build --release
```

The binary will be at `target/release/taupe`.

### Platform Compatibility

| Platform | Tested? | Works? | Best Method |
|----------|--------|--------|--------|
| Windows  | ✔ | ✔ | Build locally |
| macOS    | - | - | - |
| Linux (Ubuntu/Debian) | - | - | - |
| Linux (Arch) | ✔ | ✔ | Build locally |
| Linux (Fedora) | ✔ | ✔ | Build locally |
| Linux (OpenSUSE) | - | - | - |
