use lofty::picture::Picture;
use rodio::Source;
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
use lofty::picture::PictureType;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::Accessor;

pub enum AudioCommand {
    Load(PathBuf),
    Play,
    Pause,
    Stop,
    Seek(f32),
    SetVolume(f32),
}

#[derive(Clone, Debug)]
pub enum AudioStatus {
    Position(f32),
    Duration(f32),
    Metadata {
        title: Option<String>,
        artist: Option<String>,
        album: Option<String>,
        track_no: Option<u16>,
        disc_no: Option<u16>,
        picture: Option<(Vec<u8>, String)>,
        sample_rate_hz: Option<u32>,
        bitrate_kbps: Option<u32>,
        channels: Option<u8>,
        bit_depth: Option<u8>,
    },
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
        let mut current_state = PlaybackState::Stopped;
        let mut last_position_report = std::time::Instant::now();

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(cmd) => match cmd {
                    AudioCommand::Load(path) => {
                        sink.clear();

                        // --- read metadata ---
                        eprintln!("[AUDIO] Loading file: {:?}", path);
                        if let Ok(tagged_file) = Probe::open(&path).and_then(|p| p.read()) {
                            let tag = tagged_file
                                .primary_tag()
                                .or_else(|| tagged_file.first_tag());

                            eprintln!("[AUDIO] Tag found: {}", tag.is_some());

                            let properties = tagged_file.properties();

                            let title =
                                tag.as_ref().and_then(|t| t.title().map(|s| s.into_owned()));
                            let artist = tag
                                .as_ref()
                                .and_then(|t| t.artist().map(|s| s.into_owned()));
                            let album =
                                tag.as_ref().and_then(|t| t.album().map(|s| s.into_owned()));
                            let track_no = tag.as_ref().and_then(|t| t.track().map(|v| v as u16));
                            let disc_no = tag.as_ref().and_then(|t| t.disk().map(|v| v as u16));

                            let mut picture = None;
                            if let Some(t) = tag {
                                eprintln!("[AUDIO] Pictures count: {}", t.pictures().len());
                                for pic in t.pictures() {
                                    eprintln!("[AUDIO] Picture type: {:?}", pic.pic_type());
                                    if pic.pic_type() == PictureType::CoverFront {
                                        let data = pic.data().to_vec();
                                        let mime = pic
                                            .mime_type()
                                            .map(|m| m.to_string())
                                            .unwrap_or_else(|| "image/jpeg".to_string());

                                        eprintln!(
                                            "[AUDIO] Found CoverFront: {} bytes, mime: {}",
                                            data.len(),
                                            mime
                                        );
                                        picture = Some((data, mime));
                                        break;
                                    }
                                }
                            }
                            if picture.is_none() {
                                eprintln!("[AUDIO] No CoverFront picture found");
                            }

                            let sample_rate_hz = properties.sample_rate();
                            let bitrate_kbps = properties.audio_bitrate();
                            let channels = properties.channels();
                            let bit_depth = properties.bit_depth();

                            eprintln!("[AUDIO] Sending metadata, picture: {}", picture.is_some());
                            let _ = status_tx.send(AudioStatus::Metadata {
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
                            });
                        }

                        // --- load audio ---
                        if let Ok(file) = File::open(&path) {
                            if let Ok(source) = rodio::Decoder::try_from(file) {
                                let duration = source.total_duration();
                                sink.append(source);
                                sink.play();
                                current_state = PlaybackState::Playing;

                                if let Some(dur) = duration {
                                    let _ =
                                        status_tx.send(AudioStatus::Duration(dur.as_secs_f32()));
                                }
                            }
                        }
                    }
                    AudioCommand::Play => {
                        sink.play();
                        current_state = PlaybackState::Playing;
                    }
                    AudioCommand::Pause => {
                        sink.pause();
                        current_state = PlaybackState::Paused;
                    }
                    AudioCommand::Stop => {
                        sink.pause();
                        let _ = sink.try_seek(Duration::from_secs(0));
                        current_state = PlaybackState::Stopped;
                        let _ = status_tx.send(AudioStatus::Position(0.0));
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

            if current_state == PlaybackState::Playing
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
