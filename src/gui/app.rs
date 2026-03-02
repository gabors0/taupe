use crate::audio::{AudioCommand, AudioStatus, PlaybackState};
use ::image::ImageFormat;
use iced::widget::{
    Row, Space, button, column, container, image, mouse_area, row, rule, scrollable, slider, svg,
    table, text,
};
use iced::{Alignment, Background, Color, Element, Length};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;

const BG: Color = Color::from_rgb(0.212, 0.188, 0.169);
const BG_ALT: Color = Color::from_rgb(0.388, 0.361, 0.333);
const TEXT: Color = Color::from_rgb(0.851, 0.851, 0.851);
const TEXT_ALT: Color = Color::from_rgb(0.682, 0.631, 0.596);
const GREEN: Color = Color::from_rgb(0.604, 0.800, 0.612);
// const WARNING: Color = Color::from_rgb(0.855, 0.851, 0.525);
// const DANGER: Color = Color::from_rgb(0.847, 0.584, 0.584);

fn scan_audio_files(dir: &std::path::Path) -> Vec<PathBuf> {
    const AUDIO_EXT: &[&str] = &["flac", "mp3", "wav", "ogg", "m4a"];
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| AUDIO_EXT.contains(&e.to_ascii_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .collect();
    files.sort();
    files
}

#[derive(Clone)]
struct TrackInfo {
    index: usize,
    path: PathBuf,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    track_no: Option<u16>,
    duration_secs: Option<f32>,
}

fn scan_track_metadata(files: &[PathBuf]) -> Vec<TrackInfo> {
    files
        .iter()
        .enumerate()
        .map(|(index, path)| {
            if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
                let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag());
                let properties = tagged_file.properties();
                TrackInfo {
                    index,
                    path: path.clone(),
                    title: tag.as_ref().and_then(|t| t.title().map(|s| s.into_owned())),
                    artist: tag.as_ref().and_then(|t| t.artist().map(|s| s.into_owned())),
                    album: tag.as_ref().and_then(|t| t.album().map(|s| s.into_owned())),
                    track_no: tag.as_ref().and_then(|t| t.track().map(|v| v as u16)),
                    duration_secs: Some(properties.duration().as_secs_f32()),
                }
            } else {
                TrackInfo {
                    index,
                    path: path.clone(),
                    title: None,
                    artist: None,
                    album: None,
                    track_no: None,
                    duration_secs: None,
                }
            }
        })
        .collect()
}

fn icon(path: &str) -> svg::Svg<'_> {
    svg(path)
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(|_theme, _status| svg::Style { color: Some(BG) })
}

use rfd::FileDialog;
use std::path::PathBuf;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

pub struct App {
    audio_cmd: Sender<AudioCommand>,
    pub status_rx: Rc<RefCell<Receiver<AudioStatus>>>,
    current_file: Option<String>,
    playlist: Vec<PathBuf>,
    playlist_index: Option<usize>,
    tracks: Vec<TrackInfo>,
    selected_index: Option<usize>,
    state: PlaybackState,
    volume: f32,
    /// The actual playback position reported by the audio thread (seconds).
    position: f32,
    /// The seek bar's visual position while dragging (seconds). Equals `position` when not dragging.
    seek_position: f32,
    /// True while the user is actively dragging the seek slider.
    is_seeking: bool,
    duration: f32,
    /// --- metadata ---
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    track_no: Option<u16>,
    disc_no: Option<u16>,
    picture_handle: Option<image::Handle>,
    sample_rate_hz: Option<u32>,
    bitrate_kbps: Option<u32>,
    channels: Option<u8>,
    bit_depth: Option<u8>,
    file_format: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadPressed,
    LoadFolderPressed,
    PlayPressed,
    PausePressed,
    StopPressed,
    VolValueChanged(f32),
    /// Slider dragged – only updates the visual position, does not seek.
    SeekMoved(f32),
    /// Slider released – actually performs the seek.
    SeekReleased,
    PrevPressed,
    NextPressed,
    PlaylistRowClicked(usize),
    PlaylistRowDoubleClicked(usize),
    Tick,
}

impl App {
    pub fn new(
        audio_cmd: Sender<AudioCommand>,
        status_rx: Rc<RefCell<Receiver<AudioStatus>>>,
    ) -> Self {
        App {
            audio_cmd,
            status_rx,
            current_file: None,
            playlist: Vec::new(),
            playlist_index: None,
            tracks: Vec::new(),
            selected_index: None,
            state: PlaybackState::Stopped,
            volume: 0.5,
            position: 0.0,
            seek_position: 0.0,
            is_seeking: false,
            duration: 0.0,
            title: None,
            artist: None,
            album: None,
            track_no: None,
            disc_no: None,
            picture_handle: None,
            sample_rate_hz: None,
            bitrate_kbps: None,
            channels: None,
            bit_depth: None,
            file_format: None,
        }
    }
}

pub fn update(app: &mut App, message: Message) {
    match message {
        Message::LoadPressed => {
            if let Some(path) = FileDialog::new()
                .add_filter("audio", &["flac", "mp3", "wav", "ogg", "m4a"])
                .set_directory("/")
                .pick_file()
            {
                if let Some(dir) = path.parent() {
                    app.playlist = scan_audio_files(dir);
                    app.playlist_index = app.playlist.iter().position(|p| p == &path);
                } else {
                    app.playlist = vec![path.clone()];
                    app.playlist_index = Some(0);
                }
                app.tracks = scan_track_metadata(&app.playlist);
                app.selected_index = app.playlist_index;
                app.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                let _ = app.audio_cmd.send(AudioCommand::Load(path));
                app.state = PlaybackState::Playing;
                app.position = 0.0;
                app.seek_position = 0.0;
                app.is_seeking = false;
            }
        }
        Message::LoadFolderPressed => {
            if let Some(folder) = FileDialog::new().set_directory("/").pick_folder() {
                let files = scan_audio_files(&folder);
                if !files.is_empty() {
                    let path = files[0].clone();
                    app.playlist = files;
                    app.playlist_index = Some(0);
                    app.tracks = scan_track_metadata(&app.playlist);
                    app.selected_index = Some(0);
                    app.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                    let _ = app.audio_cmd.send(AudioCommand::Load(path));
                    app.state = PlaybackState::Playing;
                    app.position = 0.0;
                    app.seek_position = 0.0;
                    app.is_seeking = false;
                }
            }
        }
        Message::PrevPressed => {
            if let Some(idx) = app.playlist_index
                && idx > 0
            {
                let new_idx = idx - 1;
                let path = app.playlist[new_idx].clone();
                app.playlist_index = Some(new_idx);
                app.selected_index = Some(new_idx);
                app.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                let _ = app.audio_cmd.send(AudioCommand::Load(path));
                app.state = PlaybackState::Playing;
                app.position = 0.0;
                app.seek_position = 0.0;
                app.is_seeking = false;
            }
        }
        Message::NextPressed => {
            if let Some(idx) = app.playlist_index
                && idx + 1 < app.playlist.len()
            {
                let new_idx = idx + 1;
                let path = app.playlist[new_idx].clone();
                app.playlist_index = Some(new_idx);
                app.selected_index = Some(new_idx);
                app.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                let _ = app.audio_cmd.send(AudioCommand::Load(path));
                app.state = PlaybackState::Playing;
                app.position = 0.0;
                app.seek_position = 0.0;
                app.is_seeking = false;
            }
        }
        Message::PlayPressed => {
            let _ = app.audio_cmd.send(AudioCommand::Play);
            app.state = PlaybackState::Playing;
        }
        Message::PausePressed => {
            let _ = app.audio_cmd.send(AudioCommand::Pause);
            app.state = PlaybackState::Paused;
        }
        Message::StopPressed => {
            let _ = app.audio_cmd.send(AudioCommand::Stop);
            app.state = PlaybackState::Stopped;
            app.position = 0.0;
            app.seek_position = 0.0;
            app.is_seeking = false;
        }
        Message::VolValueChanged(value) => {
            app.volume = value;
            let _ = app.audio_cmd.send(AudioCommand::SetVolume(value));
        }
        Message::SeekMoved(secs) => {
            // Only update the visual position while dragging; don't seek yet.
            app.is_seeking = true;
            app.seek_position = secs;
        }
        Message::SeekReleased => {
            // Send the seek command with position converted to milliseconds.
            app.is_seeking = false;
            app.position = app.seek_position;
            let _ = app
                .audio_cmd
                .send(AudioCommand::Seek(app.seek_position * 1000.0));
        }
        Message::PlaylistRowClicked(idx) => {
            app.selected_index = Some(idx);
        }
        Message::PlaylistRowDoubleClicked(idx) => {
            let path = app.playlist[idx].clone();
            app.playlist_index = Some(idx);
            app.selected_index = Some(idx);
            app.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
            let _ = app.audio_cmd.send(AudioCommand::Load(path));
            app.state = PlaybackState::Playing;
            app.position = 0.0;
            app.seek_position = 0.0;
            app.is_seeking = false;
        }
        Message::Tick => {
            // Drain all pending status updates from the audio thread.
            let updates: Vec<_> = {
                let rx = app.status_rx.borrow();
                std::iter::from_fn(|| rx.try_recv().ok()).collect()
            };
            for status in updates {
                match status {
                    AudioStatus::Position(pos) => {
                        app.position = pos;
                        if !app.is_seeking {
                            app.seek_position = pos;
                        }
                    }
                    AudioStatus::Duration(dur) => app.duration = dur,
                    AudioStatus::PlaybackEnded => {
                        app.state = PlaybackState::Stopped;
                        app.is_seeking = false;
                        app.seek_position = app.position;
                    }
                    AudioStatus::Metadata {
                        title,
                        artist,
                        album,
                        track_no,
                        disc_no,
                        picture,
                        sample_rate_hz,
                        bitrate_kbps,
                        channels,
                        bit_depth,
                        file_format,
                    } => {
                        eprintln!("[GUI] Received metadata, picture: {}", picture.is_some());
                        app.title = title;
                        app.artist = artist;
                        app.album = album;
                        app.track_no = track_no;
                        app.disc_no = disc_no;
                        app.picture_handle = picture.map(|(data, _mime)| {
                            eprintln!("[GUI] Creating image handle from {} bytes", data.len());
                            if let Ok(img) = ::image::load_from_memory(&data) {
                                let resized = img.resize_exact(
                                    80,
                                    80,
                                    ::image::imageops::FilterType::Lanczos3,
                                );
                                let mut buffer = Vec::new();
                                let mut cursor = std::io::Cursor::new(&mut buffer);
                                if resized.write_to(&mut cursor, ImageFormat::Png).is_ok() {
                                    return image::Handle::from_bytes(buffer);
                                }
                            }
                            image::Handle::from_bytes(data)
                        });
                        app.sample_rate_hz = sample_rate_hz;
                        app.bitrate_kbps = bitrate_kbps;
                        app.channels = channels;
                        app.bit_depth = bit_depth;
                        app.file_format = file_format;
                    }
                }
            }
        }
    }
}

fn format_time(seconds: f32) -> String {
    let secs = seconds as u32;
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    // -- seperator ----------------------
    let separator = || {
        rule::horizontal(2).style(|_theme| rule::Style {
            color: BG_ALT,
            fill_mode: rule::FillMode::Full,
            radius: 0.0.into(),
            snap: true,
        })
    };

    // -- pre-build widgets ----------------------------------------------------
    let load_btn = button(icon("assets/icons/load.svg"))
        .on_press(Message::LoadPressed)
        .height(32);

    let load_folder_btn = button(icon("assets/icons/folder.svg"))
        .on_press(Message::LoadFolderPressed)
        .height(32);

    let can_prev = app.playlist_index.is_some_and(|i| i > 0);
    let can_next = app
        .playlist_index
        .is_some_and(|i| i + 1 < app.playlist.len());

    let prev_btn = button(icon("assets/icons/prev.svg"))
        .on_press_maybe(can_prev.then_some(Message::PrevPressed))
        .height(32);

    let next_btn = button(icon("assets/icons/next.svg"))
        .on_press_maybe(can_next.then_some(Message::NextPressed))
        .height(32);

    let play_pause_btn = match app.state {
        PlaybackState::Playing => button(icon("assets/icons/pause.svg"))
            .on_press(Message::PausePressed)
            .height(32),
        PlaybackState::Paused | PlaybackState::Stopped => button(icon("assets/icons/play.svg"))
            .on_press(Message::PlayPressed)
            .height(32),
    };

    let stop_btn = button(icon("assets/icons/stop.svg"))
        .on_press(Message::StopPressed)
        .height(32);

    let vol_slider = slider(0.0..=1.0, app.volume, Message::VolValueChanged)
        .step(0.01)
        .width(Length::Fill);
    let vol_label = text(format!("{:.0}%", app.volume * 100.0)).color(TEXT_ALT);
    let volume_block = row![vol_slider, vol_label]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Fixed(140.0));

    let seek_max = if app.duration > 0.0 {
        app.duration
    } else {
        0.0
    };
    let seek_slider = slider(0.0..=seek_max, app.seek_position, Message::SeekMoved)
        .on_release(Message::SeekReleased)
        .step(0.01)
        .width(Length::Fill);

    // -- row 1 -------------------------------------------------
    let format_label = app.file_format.as_deref().unwrap_or("<format>");

    let mut now_playing_children: Vec<Element<'_, Message>> = Vec::new();

    // if no album art, no space taken up
    if let Some(handle) = &app.picture_handle {
        now_playing_children.push(
            image(handle.clone())
                .width(Length::Fixed(80.0))
                .height(Length::Fixed(80.0))
                .into(),
        );
    }

    now_playing_children.push(
        column![
            text(app.title.as_deref().unwrap_or("<no title>"))
                .color(TEXT)
                .size(18),
            text(app.artist.as_deref().unwrap_or("<no artist>")).color(TEXT_ALT),
            text(app.album.as_deref().unwrap_or("<no album>")).color(TEXT_ALT),
        ]
        .spacing(4)
        .into(),
    );

    now_playing_children.push(Space::new().width(Length::Fill).into());

    // format / bitrate / sample rate
    now_playing_children.push(
        container(
            column![
                text(format_label).color(TEXT_ALT),
                text(format!("{} kbps", app.bitrate_kbps.unwrap_or(0))).color(TEXT_ALT),
                text(format!("{} Hz", app.sample_rate_hz.unwrap_or(0))).color(TEXT_ALT),
            ]
            .spacing(4)
            .align_x(Alignment::End),
        )
        .width(Length::Fixed(100.0))
        .align_right(Length::Fill)
        .into(),
    );

    let now_playing_row = Row::with_children(now_playing_children)
        .spacing(12)
        .align_y(Alignment::Center);

    // -- row 2 -----------------------------
    //   [seek -----o--------------------------------- pos/dur][vol% ----o-]
    //   |     spacer     [Load] [..] [Play/Pause] [Stop]       spacer     |
    let buttons_row = row![
        load_btn,
        load_folder_btn,
        prev_btn,
        play_pause_btn,
        stop_btn,
        next_btn,
        Space::new().width(Length::Fill),
        volume_block,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let display_position = if app.is_seeking {
        app.seek_position
    } else {
        app.position
    };

    let sliders_row = row![
        seek_slider,
        text(format!(
            "{}/{}",
            format_time(display_position),
            format_time(app.duration)
        ))
        .color(TEXT_ALT),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let controls_block = column![sliders_row, buttons_row].spacing(6);

    // -- row 3: playlist table ----------------------------------------
    let playing_idx = app.playlist_index;
    let selected_idx = app.selected_index;

    fn row_bg(idx: usize, playing_idx: Option<usize>, selected_idx: Option<usize>) -> Option<Background> {
        if Some(idx) == playing_idx {
            Some(Background::Color(Color::from_rgba(0.604, 0.800, 0.612, 0.15)))
        } else if Some(idx) == selected_idx {
            Some(Background::Color(BG_ALT))
        } else {
            None
        }
    }

    let playlist_table = table(
        [
            table::column(text("#").color(TEXT_ALT), move |track: TrackInfo| {
                let idx = track.index;
                let bg = row_bg(idx, playing_idx, selected_idx);
                let num = track.track_no.map_or(idx + 1, |n| n as usize);
                let color = if Some(idx) == playing_idx { GREEN } else { TEXT_ALT };
                mouse_area(
                    container(text(format!("{num}")).color(color))
                        .width(Length::Fill)
                        .padding([5, 10])
                        .style(move |_| iced::widget::container::Style {
                            background: bg,
                            ..Default::default()
                        }),
                )
                .on_press(Message::PlaylistRowClicked(idx))
                .on_double_click(Message::PlaylistRowDoubleClicked(idx))
            })
            .width(Length::Fixed(40.0))
            .align_x(Alignment::End),
            table::column(text("Title").color(TEXT_ALT), move |track: TrackInfo| {
                let idx = track.index;
                let bg = row_bg(idx, playing_idx, selected_idx);
                let label_str = track.title.unwrap_or_else(|| {
                    track
                        .path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("?")
                        .to_string()
                });
                let color = if Some(idx) == playing_idx { GREEN } else { TEXT };
                mouse_area(
                    container(text(label_str).color(color))
                        .width(Length::Fill)
                        .padding([5, 10])
                        .style(move |_| iced::widget::container::Style {
                            background: bg,
                            ..Default::default()
                        }),
                )
                .on_press(Message::PlaylistRowClicked(idx))
                .on_double_click(Message::PlaylistRowDoubleClicked(idx))
            })
            .width(Length::Fill),
            table::column(text("Artist").color(TEXT_ALT), move |track: TrackInfo| {
                let idx = track.index;
                let bg = row_bg(idx, playing_idx, selected_idx);
                mouse_area(
                    container(text(track.artist.unwrap_or_default()).color(TEXT_ALT))
                        .width(Length::Fill)
                        .padding([5, 10])
                        .style(move |_| iced::widget::container::Style {
                            background: bg,
                            ..Default::default()
                        }),
                )
                .on_press(Message::PlaylistRowClicked(idx))
                .on_double_click(Message::PlaylistRowDoubleClicked(idx))
            })
            .width(Length::FillPortion(2)),
            table::column(text("Album").color(TEXT_ALT), move |track: TrackInfo| {
                let idx = track.index;
                let bg = row_bg(idx, playing_idx, selected_idx);
                mouse_area(
                    container(text(track.album.unwrap_or_default()).color(TEXT_ALT))
                        .width(Length::Fill)
                        .padding([5, 10])
                        .style(move |_| iced::widget::container::Style {
                            background: bg,
                            ..Default::default()
                        }),
                )
                .on_press(Message::PlaylistRowClicked(idx))
                .on_double_click(Message::PlaylistRowDoubleClicked(idx))
            })
            .width(Length::FillPortion(2)),
            table::column(text("Duration").color(TEXT_ALT), move |track: TrackInfo| {
                let idx = track.index;
                let bg = row_bg(idx, playing_idx, selected_idx);
                let dur_str = track
                    .duration_secs
                    .map_or_else(|| "—".to_string(), format_time);
                mouse_area(
                    container(text(dur_str).color(TEXT_ALT))
                        .width(Length::Fill)
                        .padding([5, 10])
                        .style(move |_| iced::widget::container::Style {
                            background: bg,
                            ..Default::default()
                        }),
                )
                .on_press(Message::PlaylistRowClicked(idx))
                .on_double_click(Message::PlaylistRowDoubleClicked(idx))
            })
            .width(Length::Fixed(60.0))
            .align_x(Alignment::End),
        ],
        app.tracks.iter().cloned(),
    )
    .width(Length::Fill)
    .padding(0)
    .separator_x(0.0)
    .separator_y(0.0);

    let playlist_view = scrollable(playlist_table).height(Length::Fill);

    // -- main ---------------------------------------------------------
    column![
        now_playing_row,
        separator(),
        controls_block,
        separator(),
        playlist_view,
    ]
    .spacing(16)
    .padding(16)
    .into()
}
