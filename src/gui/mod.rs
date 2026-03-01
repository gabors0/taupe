mod app;

pub use app::{App, Message, view};

pub fn update(app: &mut App, message: Message) -> iced::Task<Message> {
    app::update(app, message);
    iced::Task::none()
}
