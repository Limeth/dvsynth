use iced::{button, window, text_input, Length, Button, Column, Text, Application, Command, Settings};
use style::*;
use widgets::*;

pub mod style;
pub mod widgets;

struct Counter {
    text_input_state: text_input::State,
    text_input_value: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    UpdateTextInput(String),
}

macro_rules! ui_field {
    {
        name: $name:expr,
        state: $state:expr,
        placeholder: $placeholder:expr,
        value: $value:expr,
        on_change: $on_change:expr,
        theme: $theme:expr$(,)?
    } => {{
        iced::Container::new(
            iced::Row::new()
                .push(
                    iced::Text::new($name)
                        .size(14)
                )
                .push(
                    iced::TextInput::new($state, $placeholder, $value, $on_change)
                        .size(14)
                        .style($theme)
                )
                .spacing(4) // TODO
        )
        .style($theme)
    }}
}

impl Application for Counter {
    type Executor = iced::executor::Null;
    type Message = Message;
    type Flags = (); // The data needed to initialize your Application.

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            Self {
                text_input_state: Default::default(),
                text_input_value: Default::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("A cool application")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::UpdateTextInput(new_value) => {
                self.text_input_value = new_value;
            }
        }

        Command::none()
    }

    fn view(&mut self) -> iced::Element<Message> {
        // let style: Box<dyn Style> = Box::new(StyleLight);
        let theme = style::Theme::Dark;
        // We use a column: a simple vertical layout
        iced::Element::new(Column::new()
            .push(
                iced::Container::new(
                    Margin::new(
                        iced::Element::new(
                            iced::Container::new(
                                Text::new("Test Node - Node Type")
                                    .size(16)
                            )
                        ),
                        Spacing {
                            up: 4,
                            left: 8,
                            down: 4,
                            right: 8,
                        }
                    )
                )
                .width(Length::Fill)
                .style(theme)
            )
            .push(
                ui_field! {
                    name: "Test Text Input",
                    state: &mut self.text_input_state,
                    placeholder: "Placeholder",
                    value: &self.text_input_value,
                    on_change: |new_value| {
                        Message::UpdateTextInput(new_value.to_string())
                    },
                    theme: theme,
                }
            ))
    }
}

fn main() {
    Counter::run(
        Settings {
            window: window::Settings {
                icon: None, // TODO
                ..window::Settings::default()
            },
            ..Settings::default()
        }
    ).unwrap();
}
