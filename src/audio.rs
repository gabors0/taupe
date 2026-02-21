use rodio::Source;
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub enum AudioCommand {
    Load(PathBuf),
    Play,
    Pause,
    Stop,
    Seek(f32),
    SetVolume(f32),
}

#[derive(Clone, Copy, Debug)]
pub enum AudioStatus {
    Position(f32),
    Duration(f32),
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub fn spawn_audio_thread(
    status_tx: Sender<AudioStatus>,
) -> (Sender<AudioCommand>, JoinHandle<()>) {
    let (cmd_tx, cmd_rx): (Sender<AudioCommand>, Receiver<AudioCommand>) =
        std::sync::mpsc::channel();

    let handle = thread::spawn(move || {
        let stream =
            rodio::OutputStreamBuilder::open_default_stream().expect("Failed to open audio stream");
        let sink = rodio::Sink::connect_new(stream.mixer());
        let mut _current_state = PlaybackState::Stopped;
        let mut last_position_report = std::time::Instant::now();

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(cmd) => match cmd {
                    AudioCommand::Load(path) => {
                        sink.clear();
                        if let Ok(file) = File::open(&path) {
                            if let Ok(source) = rodio::Decoder::try_from(file) {
                                let duration = source.total_duration();
                                sink.append(source);
                                sink.play();
                                _current_state = PlaybackState::Playing;

                                if let Some(dur) = duration {
                                    let _ =
                                        status_tx.send(AudioStatus::Duration(dur.as_secs_f32()));
                                }
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
                        let _ = status_tx.send(AudioStatus::Position(0.0));
                        let _ = status_tx.send(AudioStatus::Duration(0.0));
                    }
                    AudioCommand::SetVolume(vol) => {
                        sink.set_volume(vol);
                    }
                    AudioCommand::Seek(millis) => {
                        let _ = sink.try_seek(Duration::from_millis(millis as u64));
                    }
                },
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }

            if _current_state == PlaybackState::Playing
                && last_position_report.elapsed() > Duration::from_millis(250)
            {
                let pos = sink.get_pos().as_secs_f32();
                let _ = status_tx.send(AudioStatus::Position(pos));
                last_position_report = std::time::Instant::now();
            }
        }
    });

    (cmd_tx, handle)
}
