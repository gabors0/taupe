mod audio;
mod gui;

use audio::spawn_audio_thread;

fn main() -> iced::Result {
    let (audio_cmd, _audio_handle) = spawn_audio_thread();

    iced::application("Taupe", gui::update, gui::view)
        .run_with(|| (gui::App::new(audio_cmd), iced::Task::none()))
}
