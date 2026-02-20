mod audio;
mod gui;

use audio::spawn_audio_thread;
use iced::Color;
use iced::theme::Palette;

fn custom_palette() -> Palette {
    Palette {
        background: Color::from_rgb(0.12, 0.11, 0.10),
        text: Color::from_rgb(0.88, 0.86, 0.84),
        primary: Color::from_rgb(0.702, 0.576, 0.463),
        success: Color::from_rgb(0.48, 0.54, 0.41),
        danger: Color::from_rgb(0.64, 0.36, 0.31),
    }
}

fn main() -> iced::Result {
    let (audio_cmd, _audio_handle) = spawn_audio_thread();
    let theme = iced::Theme::custom("Custom Dark".into(), custom_palette());

    iced::application("Taupe", gui::update, gui::view)
        .theme(move |_| theme.clone())
        .run_with(|| (gui::App::new(audio_cmd), iced::Task::none()))
}
