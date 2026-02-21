use crate::audio::{AudioCommand, AudioStatus, PlaybackState};
use iced::widget::{button, center, column, row, slider, svg, text};
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

    column![
        file_text,
        row![load_btn, play_pause_btn, stop_btn].spacing(10),
        row![volume_slider, text("Volume")].spacing(10),
        row![
            seek_slider,
            text(format!("{:.2}s/{:.2}s", app.position, app.duration))
        ]
        .spacing(10),
    ]
    .spacing(20)
    .padding(20)
    .into()
}
