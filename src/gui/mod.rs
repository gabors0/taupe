mod app;

pub use app::{view, App, Message};

pub fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    app::update(app, message);
    iced::Task::none()
}
