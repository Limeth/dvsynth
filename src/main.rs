#![feature(const_fn_floating_point_arithmetic)]

use iced::{button, window, text_input, Point, Align, VerticalAlignment, HorizontalAlignment, Length, Button, Column, Text, Application, Command, Settings};
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

macro_rules! margin {
    {
        element: $element:expr,
        spacing: $spacing:expr$(,)?
    } => {{
        use iced::{Row, Column, Space, Length};
        Column::new()
            .push(Space::with_height(Length::Units($spacing.up)))
            .push(
                Row::new()
                    .push(Space::with_width(Length::Units($spacing.left)))
                    .push($element)
                    .push(Space::with_width(Length::Units($spacing.right)))
            )
            .push(Space::with_height(Length::Units($spacing.down)))
    }}
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
            // Margin::new(
            margin! {
                element: iced::Row::new()
                    .spacing(consts::SPACING_HORIZONTAL)
                    .align_items(Align::Center)
                    .push(
                        iced::Text::new($name)
                            .size(14)
                    )
                    .push(
                        iced::TextInput::new($state, $placeholder, $value, $on_change)
                            .size(14)
                            // .width(Length::Shrink)
                            .padding(consts::SPACING_VERTICAL)
                            .style($theme)
                    ),
                spacing: consts::SPACING,
            }
            // )
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
        iced::Element::new(
            FloatingPanes::new()
                .push(
                    FloatingPane::builder(
                        Column::new()
                            .width(Length::Units(256))
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
                            )
                    )
                    .position([10, 20])
                    .title(Some("First"))
                    .title_size(Some(16))
                    .title_margin(consts::SPACING)
                    .pane_style(Some(theme))
                    .build(),
                )
                .push(
                    FloatingPane::builder(
                        Column::new()
                            .width(Length::Units(256))
                            .push(
                                iced::Container::new(
                                    // Margin::new(
                                    margin! {
                                        element: iced::Container::new(
                                            Text::new("Test Node - Node Type")
                                                .size(16)
                                        ),
                                        spacing: consts::SPACING,
                                    }
                                    // )
                                )
                                .width(Length::Fill)
                                .style(theme)
                            )
                    )
                    .position([100, 100])
                    .title(Some("Second"))
                    .title_size(Some(16))
                    .title_margin(consts::SPACING)
                    .pane_style(Some(theme))
                    .build(),
                )
        )
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
