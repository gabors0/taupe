use crate::audio::{AudioCommand, AudioStatus, PlaybackState};
use ::image::ImageFormat;
use iced::widget::{Row, Space, button, column, container, image, row, rule, slider, svg, text};
use iced::{Alignment, Color, Element, Length};

const BG: Color = Color::from_rgb(0.212, 0.188, 0.169);
const BG_ALT: Color = Color::from_rgb(0.388, 0.361, 0.333);
const TEXT: Color = Color::from_rgb(0.851, 0.851, 0.851);
const TEXT_ALT: Color = Color::from_rgb(0.682, 0.631, 0.596);
// const SUCCESS: Color = Color::from_rgb(0.48, 0.54, 0.41);
// const WARNING: Color = Color::from_rgb(0.855, 0.851, 0.525);
// const DANGER: Color = Color::from_rgb(0.847, 0.584, 0.584);

fn icon(path: &str) -> svg::Svg<'_> {
    svg(path)
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .style(|_theme, _status| svg::Style { color: Some(BG) })
}

use rfd::FileDialog;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};

pub struct App {
    audio_cmd: Sender<AudioCommand>,
    pub status_rx: Rc<RefCell<Receiver<AudioStatus>>>,
    current_file: Option<String>,
    state: PlaybackState,
    volume: f32,
    /// The actual playback position reported by the audio thread (seconds).
    position: f32,
    /// The seek bar's visual position while dragging (seconds). Equals `position` when not dragging.
    seek_position: f32,
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
    PlayPressed,
    PausePressed,
    StopPressed,
    VolValueChanged(f32),
    /// Slider dragged – only updates the visual position, does not seek.
    SeekMoved(f32),
    /// Slider released – actually performs the seek.
    SeekReleased,
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
            state: PlaybackState::Stopped,
            volume: 0.5,
            position: 0.0,
            seek_position: 0.0,
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
                app.current_file = path.file_name().map(|n| n.to_string_lossy().to_string());
                let _ = app.audio_cmd.send(AudioCommand::Load(path));
                app.state = PlaybackState::Playing;
                app.position = 0.0;
                app.seek_position = 0.0;
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
            app.duration = 0.0;
            app.title = None;
            app.artist = None;
            app.album = None;
            app.track_no = None;
            app.disc_no = None;
            app.picture_handle = None;
            app.file_format = None;
        }
        Message::VolValueChanged(value) => {
            app.volume = value;
            let _ = app.audio_cmd.send(AudioCommand::SetVolume(value));
        }
        Message::SeekMoved(secs) => {
            // Only update the visual position while dragging; don't seek yet.
            app.seek_position = secs;
        }
        Message::SeekReleased => {
            // Send the seek command with position converted to milliseconds.
            app.position = app.seek_position;
            let _ = app
                .audio_cmd
                .send(AudioCommand::Seek(app.seek_position * 1000.0));
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
                        app.seek_position = pos;
                    }
                    AudioStatus::Duration(dur) => app.duration = dur,
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
    let load_btn = button("Load").on_press(Message::LoadPressed).height(32);

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
    let format_label = app.file_format.as_deref().unwrap_or("<format label>");

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
    //   |      spacer      [Load] [Play/Pause] [Stop]        spacer       |
    let buttons_row = row![
        Space::new().width(Length::Fill),
        load_btn,
        play_pause_btn,
        stop_btn,
        Space::new().width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let sliders_row = row![
        seek_slider,
        text(format!("{:.1}/{:.1}", app.position, app.duration)).color(TEXT_ALT),
        volume_block,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let controls_block = column![sliders_row, buttons_row].spacing(6);

    // -- row 3 (under construction) ----------------------------------------
    let empty = Space::new().height(Length::Fill);

    // -- main ---------------------------------------------------------
    column![
        now_playing_row,
        separator(),
        controls_block,
        separator(),
        empty,
    ]
    .spacing(16)
    .padding(16)
    .into()
}
