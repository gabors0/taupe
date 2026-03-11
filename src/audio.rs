use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::file::FileType;
use lofty::file::TaggedFileExt;
use lofty::mp4::{Mp4Codec, Mp4File};
use lofty::picture::PictureType;
use lofty::probe::Probe;
use lofty::tag::{Accessor, Tag};
use rodio::Source;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

macro_rules! debug_log {
    ($($arg:tt)*) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)*);
        }
    };
}

fn pick_picture_index(types: &[PictureType]) -> Option<usize> {
    if types.is_empty() {
        return None;
    }

    if let Some((idx, _)) = types
        .iter()
        .enumerate()
        .find(|(_, t)| **t == PictureType::CoverFront)
    {
        return Some(idx);
    }

    // MP4/ALAC (ilst) pictures often don't preserve picture type, and may come through as `Other`.
    Some(0)
}

fn extract_picture(tag: Option<&Tag>) -> Option<(Vec<u8>, String)> {
    let tag = tag?;
    let pictures = tag.pictures();

    let types: Vec<PictureType> = pictures.iter().map(|p| p.pic_type()).collect();
    let idx = pick_picture_index(&types)?;
    let pic = pictures.get(idx)?;

    let data = pic.data().to_vec();
    let mime = pic
        .mime_type()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "image/jpeg".to_string());

    debug_log!(
        "[AUDIO] Selected picture: idx={}, type={:?}, bytes={}, mime={}",
        idx,
        pic.pic_type(),
        data.len(),
        mime
    );

    Some((data, mime))
}

fn send_metadata(status_tx: &Sender<AudioStatus>, path: &Path) {
    debug_log!("[AUDIO] Reading metadata: {:?}", path);

    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());
        let properties = tagged_file.properties();

        let title = tag.as_ref().and_then(|t| t.title().map(|s| s.into_owned()));
        let artist = tag
            .as_ref()
            .and_then(|t| t.artist().map(|s| s.into_owned()));
        let album = tag.as_ref().and_then(|t| t.album().map(|s| s.into_owned()));
        let track_no = tag.as_ref().and_then(|t| t.track().map(|v| v as u16));
        let disc_no = tag.as_ref().and_then(|t| t.disk().map(|v| v as u16));

        let picture = extract_picture(tag);

        let sample_rate_hz = properties.sample_rate();
        let bitrate_kbps = properties.audio_bitrate();
        let channels = properties.channels();
        let bit_depth = properties.bit_depth();
        let file_format = match tagged_file.file_type() {
            FileType::Mpeg => Some("mp3".to_string()),
            FileType::Flac => Some("flac".to_string()),
            FileType::Wav => Some("wav".to_string()),
            FileType::Vorbis => Some("ogg".to_string()),
            FileType::Mp4 => {
                let codec_name = std::fs::File::open(path)
                    .ok()
                    .and_then(|mut f| Mp4File::read_from(&mut f, ParseOptions::new()).ok())
                    .map(|mp4| match mp4.properties().codec() {
                        Mp4Codec::AAC => "AAC",
                        Mp4Codec::ALAC => "ALAC",
                        Mp4Codec::MP3 => "MP3",
                        Mp4Codec::FLAC => "FLAC",
                        _ => "mp4",
                    });
                codec_name.map(|s| s.to_string())
            }
            FileType::Mpc => Some("mpc".to_string()),
            FileType::Opus => Some("opus".to_string()),
            FileType::Ape => Some("ape".to_string()),
            FileType::Aac => Some("aac".to_string()),
            FileType::Aiff => Some("aiff".to_string()),
            FileType::Speex => Some("speex".to_string()),
            FileType::WavPack => Some("wv".to_string()),
            FileType::Custom(_) => None,
            _ => None,
        };

        debug_log!("[AUDIO] Sending metadata, picture: {}", picture.is_some());
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
            file_format,
        });
    }
}

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
    PlaybackEnded,
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
        file_format: Option<String>,
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
        sink.set_volume(0.5);
        let mut current_state = PlaybackState::Stopped;
        let mut last_position_report = std::time::Instant::now();
        let mut current_path: Option<PathBuf> = None;
        let mut current_duration: Option<Duration> = None;
        let mut was_empty = true;

        loop {
            match cmd_rx.recv_timeout(Duration::from_millis(100)) {
                Ok(cmd) => match cmd {
                    AudioCommand::Load(path) => {
                        sink.clear();

                        // --- read metadata ---
                        send_metadata(&status_tx, &path);

                        // --- load audio ---
                        if let Ok(file) = File::open(&path)
                            && let Ok(source) = rodio::Decoder::try_from(file)
                        {
                            let duration = source.total_duration();
                            sink.append(source);
                            sink.play();
                            current_state = PlaybackState::Playing;
                            current_path = Some(path);
                            current_duration = duration;

                            if let Some(dur) = duration {
                                let _ = status_tx.send(AudioStatus::Duration(dur.as_secs_f32()));
                            }
                            was_empty = false;
                        }
                    }
                    AudioCommand::Play => {
                        if sink.empty()
                            && let Some(path) = current_path.clone()
                            && let Ok(file) = File::open(&path)
                            && let Ok(source) = rodio::Decoder::try_from(file)
                        {
                            let duration = source.total_duration();
                            sink.append(source);
                            current_duration = duration;
                            let _ = status_tx.send(AudioStatus::Position(0.0));
                            if let Some(dur) = duration {
                                let _ = status_tx.send(AudioStatus::Duration(dur.as_secs_f32()));
                            }
                            was_empty = false;
                        }
                        sink.play();
                        current_state = PlaybackState::Playing;
                    }
                    AudioCommand::Pause => {
                        sink.pause();
                        current_state = PlaybackState::Paused;
                    }
                    AudioCommand::Stop => {
                        if let Some(path) = current_path.clone() {
                            sink.clear();
                            send_metadata(&status_tx, &path);

                            if let Ok(file) = File::open(&path)
                                && let Ok(source) = rodio::Decoder::try_from(file)
                            {
                                let duration = source.total_duration();
                                sink.append(source);
                                current_duration = duration;
                                current_state = PlaybackState::Stopped;
                                let _ = status_tx.send(AudioStatus::Position(0.0));
                                if let Some(dur) = duration {
                                    let _ =
                                        status_tx.send(AudioStatus::Duration(dur.as_secs_f32()));
                                }
                                was_empty = false;
                            }
                        } else {
                            sink.pause();
                            let _ = sink.try_seek(Duration::from_secs(0));
                            current_state = PlaybackState::Stopped;
                            let _ = status_tx.send(AudioStatus::Position(0.0));
                            was_empty = sink.empty();
                        }
                    }
                    AudioCommand::SetVolume(vol) => {
                        sink.set_volume(vol);
                    }
                    AudioCommand::Seek(millis) => {
                        let seek_target = Duration::from_millis(millis as u64);
                        if sink.empty()
                            && let Some(path) = current_path.clone()
                            && let Ok(file) = File::open(&path)
                            && let Ok(source) = rodio::Decoder::try_from(file)
                        {
                            let duration = source.total_duration();
                            sink.append(source);
                            current_duration = duration;
                            if let Some(dur) = duration {
                                let _ = status_tx.send(AudioStatus::Duration(dur.as_secs_f32()));
                            }
                        }

                        let _ = sink.try_seek(seek_target);

                        if current_state == PlaybackState::Playing {
                            sink.play();
                        } else {
                            sink.pause();
                        }

                        if let Some(dur) = current_duration {
                            if seek_target >= dur {
                                let _ = status_tx.send(AudioStatus::Position(dur.as_secs_f32()));
                            } else {
                                let _ = status_tx.send(AudioStatus::Position(seek_target.as_secs_f32()));
                            }
                        } else {
                            let _ = status_tx.send(AudioStatus::Position(seek_target.as_secs_f32()));
                        }
                        was_empty = sink.empty();
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

            let is_empty = sink.empty();
            if current_state == PlaybackState::Playing && !was_empty && is_empty {
                current_state = PlaybackState::Stopped;
                let end_pos = current_duration.map_or(0.0, |d| d.as_secs_f32());
                let _ = status_tx.send(AudioStatus::Position(end_pos));
                let _ = status_tx.send(AudioStatus::PlaybackEnded);
            }
            was_empty = is_empty;
        }
    });

    (cmd_tx, handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_picture_index_prefers_cover_front() {
        let types = vec![
            PictureType::Other,
            PictureType::CoverFront,
            PictureType::Other,
        ];
        assert_eq!(pick_picture_index(&types), Some(1));
    }

    #[test]
    fn pick_picture_index_falls_back_to_first() {
        let types = vec![PictureType::Other, PictureType::Other];
        assert_eq!(pick_picture_index(&types), Some(0));
    }

    #[test]
    fn pick_picture_index_none_for_empty() {
        let types: Vec<PictureType> = Vec::new();
        assert_eq!(pick_picture_index(&types), None);
    }
}
