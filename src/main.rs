mod audio;
mod gui;

use audio::spawn_audio_thread;
use gui::Message;
use iced::theme::Palette;
use iced::Color;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

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
    use audio::AudioStatus;

    let (status_tx, status_rx): (Sender<AudioStatus>, Receiver<AudioStatus>) =
        std::sync::mpsc::channel();
    let status_rx = Rc::new(RefCell::new(status_rx));
    let (audio_cmd, _audio_handle) = spawn_audio_thread(status_tx);

    let status_rx_for_init = Rc::clone(&status_rx);

    iced::application(
        move || {
            (
                gui::App::new(audio_cmd.clone(), status_rx_for_init.clone()),
                iced::Task::none(),
            )
        },
        gui::update,
        gui::view,
    )
    .subscription(|_app| {
        // Fire a Tick message every 100ms so the update loop can drain audio status updates.
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
    })
    .title("Taupe")
    .theme(theme_fn)
    .run()
}
