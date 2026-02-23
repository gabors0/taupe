use rodio::Source;
use std::fs::File;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

// Import lofty traits and types
use lofty::file::AudioFile;
use lofty::file::TaggedFileExt;
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
        sample_rate_hz: Option<u32>,
        bitrate_kbps: Option<u32>,
        channels: Option<u8>,
        bit_depth: Option<u8>,
        codec: Option<String>,
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
        let mut _current_state = PlaybackState::Stopped;
        let mut last_position_report = std::time::Instant::now();

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(cmd) => match cmd {
                    AudioCommand::Load(path) => {
                        sink.clear();

                        // --- read metadata ---
                        if let Ok(tagged_file) = Probe::open(&path).and_then(|p| p.read()) {
                            let tag = tagged_file
                                .primary_tag()
                                .or_else(|| tagged_file.first_tag());

                            #[allow(unused_assignments)]
                            let mut title: Option<String> = None;
                            #[allow(unused_assignments)]
                            let mut artist: Option<String> = None;
                            #[allow(unused_assignments)]
                            let mut album: Option<String> = None;
                            #[allow(unused_assignments)]
                            let mut track_no: Option<u16> = None;
                            #[allow(unused_assignments)]
                            let mut disc_no: Option<u16> = None;
                            #[allow(unused_assignments)]
                            let mut sample_rate_hz: Option<u32> = None;
                            #[allow(unused_assignments)]
                            let mut bitrate_kbps: Option<u32> = None;
                            #[allow(unused_assignments)]
                            let mut channels: Option<u8> = None;
                            #[allow(unused_assignments)]
                            let mut bit_depth: Option<u8> = None;
                            #[allow(unused_assignments)]
                            let mut codec: Option<String> = None;

                            if let Some(tag) = tag {
                                title = tag.title().map(|s| s.into_owned());
                                artist = tag.artist().map(|s| s.into_owned());
                                album = tag.album().map(|s| s.into_owned());
                                track_no = tag.track().map(|t| t as u16);
                                disc_no = tag.disk().map(|d| d as u16);
                            }

                            let properties = tagged_file.properties();
                            sample_rate_hz = properties.sample_rate();
                            bitrate_kbps = properties.audio_bitrate();
                            channels = properties.channels();
                            bit_depth = properties.bit_depth();
                            codec = None;

                            let _ = status_tx.send(AudioStatus::Metadata {
                                title,
                                artist,
                                album,
                                track_no,
                                disc_no,
                                sample_rate_hz,
                                bitrate_kbps,
                                channels,
                                bit_depth,
                                codec,
                            });
                        }

                        // --- load audio ---
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
                        sink.pause();
                        let _ = sink.try_seek(Duration::from_secs(0));
                        _current_state = PlaybackState::Stopped;
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
