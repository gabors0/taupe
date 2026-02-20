use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

pub enum AudioCommand {
    Load(PathBuf),
    Play,
    Pause,
    Stop,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub fn spawn_audio_thread() -> (Sender<AudioCommand>, JoinHandle<()>) {
    let (cmd_tx, cmd_rx): (Sender<AudioCommand>, Receiver<AudioCommand>) =
        std::sync::mpsc::channel();

    let handle = thread::spawn(move || {
        let stream =
            rodio::OutputStreamBuilder::open_default_stream().expect("Failed to open audio stream");
        let sink = rodio::Sink::connect_new(stream.mixer());
        let mut _current_state = PlaybackState::Stopped;

        for cmd in cmd_rx {
            match cmd {
                AudioCommand::Load(path) => {
                    sink.clear();
                    if let Ok(file) = File::open(&path) {
                        if let Ok(source) = rodio::Decoder::try_from(file) {
                            sink.append(source);
                            _current_state = PlaybackState::Playing;
                        }
                    }
                }
                AudioCommand::Play => {
                    sink.play();
                    _current_state = PlaybackState::Playing;
                }
                AudioCommand::Pause => {
                    sink.pause();
                    _current_state = PlaybackState::Paused;
                }
                AudioCommand::Stop => {
                    sink.clear();
                    _current_state = PlaybackState::Stopped;
                }
            }
        }
    });

    (cmd_tx, handle)
}
