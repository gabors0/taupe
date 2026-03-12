mod audio;
mod gui;

use audio::spawn_audio_thread;
use gui::Message;
use iced::Color;
use iced::theme::Palette;
use iced::window;
use souvlaki::{MediaControlEvent, MediaControls, PlatformConfig};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

const fn custom_palette() -> Palette {
    Palette {
        // bg from_rgb(0.212, 0.188, 0.169)
        // bg-alt from_rgb(0.388, 0.361, 0.333) todo
        // text, primary from_rgb(0.682, 0.631, 0.596)
        // text-alt from_rgb(0.851, 0.851, 0.851) todo
        // success from_rgb(0.604, 0.8, 0.612)
        // warning from_rgb(0.855, 0.851, 0.525)
        // danger from_rgb(0.847, 0.584, 0.584)
        // https://coolors.co/36302b-635c55-aea198-d9d9d9-9acc9c-dad986-d89595
        background: Color::from_rgb(0.212, 0.188, 0.169),
        text: Color::from_rgb(0.682, 0.631, 0.596),
        primary: Color::from_rgb(0.682, 0.631, 0.596),
        success: Color::from_rgb(0.48, 0.54, 0.41),
        warning: Color::from_rgb(0.855, 0.851, 0.525),
        danger: Color::from_rgb(0.847, 0.584, 0.584),
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

    let (media_event_tx, media_event_rx) = std::sync::mpsc::channel::<MediaControlEvent>();
    let media_event_rx = Rc::new(RefCell::new(media_event_rx));

    let config = PlatformConfig {
        dbus_name: "taupe",
        display_name: "Taupe",
        hwnd: None,
    };
    let controls = MediaControls::new(config).expect("Failed to create media controls");
    // NOTE: attach() is NOT called here; it is deferred until the first track is loaded.
    let controls = Rc::new(RefCell::new(controls));

    let status_rx_for_init = Rc::clone(&status_rx);
    let controls_for_init = Rc::clone(&controls);
    let media_event_rx_for_init = Rc::clone(&media_event_rx);

    iced::application(
        move || {
            (
                gui::App::new(
                    audio_cmd.clone(),
                    status_rx_for_init.clone(),
                    controls_for_init.clone(),
                    media_event_rx_for_init.clone(),
                    media_event_tx.clone(),
                ),
                iced::Task::none(),
            )
        },
        gui::update,
        gui::view,
    )
    .subscription(|_app| {
        use iced::Event;
        use iced::keyboard::{Event as KeyEvent, Key, key::Named};
        use iced::mouse::{Event as MouseEvent, ScrollDelta};

        let tick = iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick);

        let keys = iced::event::listen_with(|event, _status, _id| match event {
            Event::Keyboard(KeyEvent::KeyPressed { key, .. }) => match key {
                Key::Named(Named::Space) => Some(Message::TogglePlayPause),
                Key::Named(Named::ArrowLeft) => Some(Message::SeekBackward),
                Key::Named(Named::ArrowRight) => Some(Message::SeekForward),
                Key::Named(Named::ArrowUp) => Some(Message::VolumeUp),
                Key::Named(Named::ArrowDown) => Some(Message::VolumeDown),
                _ => None,
            },
            Event::Mouse(MouseEvent::WheelScrolled { delta }) => {
                let y = match delta {
                    ScrollDelta::Lines { y, .. } => y,
                    ScrollDelta::Pixels { y, .. } => y,
                };
                if y > 0.0 {
                    Some(Message::VolumeUp)
                } else if y < 0.0 {
                    Some(Message::VolumeDown)
                } else {
                    None
                }
            }
            _ => None,
        });

        iced::Subscription::batch([tick, keys])
    })
    .title("Taupe")
    .theme(theme_fn)
    .window(window::Settings {
        min_size: Some(iced::Size::new(500.0, 550.0)),
        ..window::Settings::default()
    })
    .run()
}
