use crate::audio::{AudioCommand, PlaybackState};
use iced::widget::{button, column, row, text};
use iced::Element;
use rfd::FileDialog;
use std::sync::mpsc::Sender;

pub struct App {
    audio_cmd: Sender<AudioCommand>,
    current_file: Option<String>,
    state: PlaybackState,
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadPressed,
    PlayPressed,
    PausePressed,
}

impl App {
    pub fn new(audio_cmd: Sender<AudioCommand>) -> Self {
        App {
            audio_cmd,
            current_file: None,
            state: PlaybackState::Stopped,
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
    }
}

pub fn view(app: &App) -> Element<'_, Message> {
    let file_text = text(if let Some(name) = &app.current_file {
        name.clone()
    } else {
        "No file loaded".to_string()
    });

    let load_btn = button("Load").on_press(Message::LoadPressed);

    let play_pause_btn = match app.state {
        PlaybackState::Playing => button("Pause").on_press(Message::PausePressed),
        PlaybackState::Paused | PlaybackState::Stopped => {
            button("Play").on_press(Message::PlayPressed)
        }
    };

    column![file_text, row![load_btn, play_pause_btn].spacing(10),]
        .spacing(20)
        .padding(20)
        .into()
}
