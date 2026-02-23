use crate::audio::{AudioCommand, AudioStatus, PlaybackState};
use iced::widget::{button, column, image, row, slider, svg, text};
use iced::{Color, Element, Length};

const BG: Color = Color::from_rgb(0.212, 0.188, 0.169);
const BG_ALT: Color = Color::from_rgb(0.388, 0.361, 0.333);
const TEXT: Color = Color::from_rgb(0.682, 0.631, 0.596);
const TEXT_ALT: Color = Color::from_rgb(0.851, 0.851, 0.851);
const SUCCESS: Color = Color::from_rgb(0.48, 0.54, 0.41);
const WARNING: Color = Color::from_rgb(0.855, 0.851, 0.525);
const DANGER: Color = Color::from_rgb(0.847, 0.584, 0.584);

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
            volume: 1.0,
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
            app.current_file = None;
            app.position = 0.0;
            app.seek_position = 0.0;
            app.duration = 0.0;
            app.title = None;
            app.artist = None;
            app.album = None;
            app.track_no = None;
            app.disc_no = None;
            app.picture_handle = None;
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
                    } => {
                        eprintln!("[GUI] Received metadata, picture: {}", picture.is_some());
                        app.title = title;
                        app.artist = artist;
                        app.album = album;
                        app.track_no = track_no;
                        app.disc_no = disc_no;
                        app.picture_handle = picture.map(|(data, _mime)| {
                            eprintln!("[GUI] Creating image handle from {} bytes", data.len());
                            image::Handle::from_bytes(data)
                        });
                        app.sample_rate_hz = sample_rate_hz;
                        app.bitrate_kbps = bitrate_kbps;
                        app.channels = channels;
                        app.bit_depth = bit_depth;
                    }
                }
            }
        }
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let file_text = text(if let Some(name) = &app.current_file {
        name.clone()
    } else {
        "No file loaded".to_string()
    });

    let load_btn = button("Load file")
        .on_press(Message::LoadPressed)
        .height(32);

    let play_pause_btn = match app.state {
        PlaybackState::Playing => button(icon("assets/pause.svg"))
            .on_press(Message::PausePressed)
            .height(32),
        PlaybackState::Paused | PlaybackState::Stopped => button(icon("assets/play.svg"))
            .on_press(Message::PlayPressed)
            .height(32),
    };

    let stop_btn = button(icon("assets/stop.svg"))
        .on_press(Message::StopPressed)
        .height(32);

    let volume_slider = slider(0.0..=1.0, app.volume, Message::VolValueChanged)
        .step(0.01)
        .width(60);

    // Use seek_position so the slider tracks dragging visually.
    // on_release fires SeekReleased when the mouse button is lifted.
    let seek_max = if app.duration > 0.0 {
        app.duration
    } else {
        0.0
    };
    let seek_slider = slider(0.0..=seek_max, app.seek_position, Message::SeekMoved)
        .on_release(Message::SeekReleased)
        .step(0.01)
        .width(120);

    let cover_image: Element<'_, Message> = if let Some(handle) = &app.picture_handle {
        image(handle.clone())
            .width(Length::Fixed(200.0))
            .height(Length::Fixed(200.0))
            .into()
    } else {
        text("No cover art").into()
    };

    column![
        file_text,
        row![load_btn, play_pause_btn, stop_btn].spacing(10),
        row![volume_slider, text("Volume")].spacing(10),
        row![
            seek_slider,
            text(format!("{:.2}s/{:.2}s", app.position, app.duration))
        ],
        cover_image,
        row![text(format!(
            "Title: {}",
            app.title.as_deref().unwrap_or("Unknown")
        )),]
        .spacing(5),
        row![text(format!(
            "Artist: {}",
            app.artist.as_deref().unwrap_or("Unknown")
        )),]
        .spacing(5),
        row![text(format!(
            "Album: {}",
            app.album.as_deref().unwrap_or("Unknown")
        )),]
        .spacing(5),
        row![
            text(format!("Track: {}", app.track_no.unwrap_or(0))),
            text(format!("Disc: {}", app.disc_no.unwrap_or(0)))
        ]
        .spacing(10),
        row![
            text(format!(
                "Sample Rate: {} Hz",
                app.sample_rate_hz.unwrap_or(0)
            )),
            text(format!("Bitrate: {} kbps", app.bitrate_kbps.unwrap_or(0)))
        ]
        .spacing(10),
        row![
            text(format!("Channels: {}", app.channels.unwrap_or(0))),
            text(format!("Bit Depth: {} bit", app.bit_depth.unwrap_or(0))),
        ]
        .spacing(10),
    ]
    .spacing(20)
    .padding(20)
    .into()
}
