mod audio;
mod gui;

use audio::spawn_audio_thread;
use iced::theme::Palette;
use iced::Color;

const fn custom_palette() -> Palette {
    Palette {
        background: Color::from_rgb(0.12, 0.11, 0.10),
        text: Color::from_rgb(0.88, 0.86, 0.84),
        primary: Color::from_rgb(0.702, 0.576, 0.463),
        success: Color::from_rgb(0.48, 0.54, 0.41),
        warning: Color::from_rgb(0.80, 0.60, 0.30),
        danger: Color::from_rgb(0.64, 0.36, 0.31),
    }
}

fn theme_fn(_state: &gui::App) -> iced::Theme {
    iced::Theme::custom("Custom Dark", custom_palette())
}

fn main() -> iced::Result {
    let (audio_cmd, _audio_handle) = spawn_audio_thread();

    iced::application(
        move || (gui::App::new(audio_cmd.clone()), iced::Task::none()),
        gui::update,
        gui::view,
    )
    .title("Taupe")
    .theme(theme_fn)
    .run()
}
