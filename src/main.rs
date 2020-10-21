#![feature(const_fn_floating_point_arithmetic)]
///
/// Task list:
/// * Change background color;
/// * Make sure that the pane title bar has a different background color than the rest of the pane;
/// * Add channel connection points;
///

use std::borrow::Cow;
use iced::{button, window, text_input, Point, Align, VerticalAlignment, HorizontalAlignment, Length, Button, Column, Text, Application, Command, Settings};
use iced_wgpu::Renderer;
use petgraph::{stable_graph::StableGraph, Directed};
use style::*;
use widgets::*;

pub mod style;
pub mod widgets;

struct Channel {
    pub title: String,
    pub description: Option<String>,
}

impl<'a> From<&'a Channel> for ChannelSlice<'a> {
    fn from(other: &'a Channel) -> Self {
        Self {
            title: &other.title,
            description: other.description.as_ref().map(String::as_str),
        }
    }
}

impl Channel {
    pub fn new(title: impl ToString) -> Self {
        Self {
            title: title.to_string(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }
}

struct NodeData {
    title: String,
    element_state: NodeElementState,
    floating_pane_state: FloatingPaneState,
    input_channels: Vec<Channel>,
    output_channels: Vec<Channel>,
}

impl NodeData {
    pub fn view(&mut self, theme: style::Theme) -> FloatingPane<'_, Message, Renderer> {
        let mut builder = NodeElement::builder(&mut self.element_state);

        for input_channel in &self.input_channels {
            builder = builder.push_input_channel(input_channel);
        }

        for output_channel in &self.output_channels {
            builder = builder.push_output_channel(output_channel);
        }

        let node_element = builder.build();

        FloatingPane::builder(
            &mut self.floating_pane_state,
            node_element,
        )
        .title(Some(&self.title))
        .title_size(Some(16))
        .title_margin(consts::SPACING)
        .pane_style(Some(theme))
        .build()
    }
}

struct EdgeData {
    channel_index_from: usize,
    channel_index_to: usize,
}

type Graph = StableGraph<
    NodeData, // Node Data
    EdgeData, // Edge Data
    Directed, // Edge Type
    u32, // Node Index
>;

struct ApplicationState {
    text_input_state: text_input::State,
    text_input_value: String,

    graph: Graph,

    floating_panes_state: FloatingPanesState,
    floating_pane_state_0: FloatingPaneState,
    floating_pane_state_1: FloatingPaneState,
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

impl Application for ApplicationState {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = (); // The data needed to initialize your Application.

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            Self {
                text_input_state: Default::default(),
                text_input_value: Default::default(),
                graph: {
                    let mut graph = Graph::new();

                    let node_a = graph.add_node(NodeData {
                        title: "Node A".to_string(),
                        element_state: Default::default(),
                        floating_pane_state: FloatingPaneState::with_position([10.0, 10.0]),
                        input_channels: vec![
                            Channel::new("In A"),
                        ],
                        output_channels: vec![
                            Channel::new("Out A"),
                            Channel::new("Outtt B"),
                        ],
                    });

                    let node_b = graph.add_node(NodeData {
                        title: "Node B".to_string(),
                        element_state: Default::default(),
                        floating_pane_state: FloatingPaneState::with_position([100.0, 10.0]),
                        input_channels: vec![
                            Channel::new("In A"),
                            Channel::new("In B"),
                            Channel::new("In C"),
                        ],
                        output_channels: vec![
                            Channel::new("Out A"),
                        ],
                    });

                    graph.add_edge(node_a, node_b, EdgeData {
                        channel_index_from: 0,
                        channel_index_to: 2,
                    });

                    graph.add_edge(node_a, node_b, EdgeData {
                        channel_index_from: 1,
                        channel_index_to: 0,
                    });

                    graph
                },
                floating_panes_state: Default::default(),
                floating_pane_state_0: FloatingPaneState::with_position([0.0, 0.0]),
                floating_pane_state_1: FloatingPaneState::with_position([100.0, 100.0]),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("DVSynth")
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

        let mut panes = FloatingPanes::new(&mut self.floating_panes_state);
        let node_indices = self.graph.node_indices().collect::<Vec<_>>();

        for (node_index, node_data) in node_indices.iter().zip(self.graph.node_weights_mut()) {
            panes = panes.push(node_data.view(theme));
        }

        panes.into()

//         // We use a column: a simple vertical layout
//         iced::Element::new(
//             FloatingPanes::new(&mut self.floating_panes_state)
//                 .push(
//                     FloatingPane::builder(
//                         &mut self.floating_pane_state_0,
//                         Column::new()
//                             .width(Length::Units(256))
//                             .push(
//                                 ui_field! {
//                                     name: "Test Text Input",
//                                     state: &mut self.text_input_state,
//                                     placeholder: "Placeholder",
//                                     value: &self.text_input_value,
//                                     on_change: |new_value| {
//                                         Message::UpdateTextInput(new_value.to_string())
//                                     },
//                                     theme: theme,
//                                 }
//                             ),
//                     )
//                     .title(Some("First"))
//                     .title_size(Some(16))
//                     .title_margin(consts::SPACING)
//                     .pane_style(Some(theme))
//                     .build(),
//                 )
//                 .push(
//                     FloatingPane::builder(
//                         &mut self.floating_pane_state_1,
//                         Column::new()
//                             .width(Length::Units(256))
//                             .push(
//                                 iced::Container::new(
//                                     // Margin::new(
//                                     margin! {
//                                         element: iced::Container::new(
//                                             Text::new("Test Node - Node Type")
//                                                 .size(16)
//                                         ),
//                                         spacing: consts::SPACING,
//                                     }
//                                     // )
//                                 )
//                                 .width(Length::Fill)
//                                 .style(theme)
//                             ),
//                     )
//                     .title(Some("Second"))
//                     .title_size(Some(16))
//                     .title_margin(consts::SPACING)
//                     .pane_style(Some(theme))
//                     .build(),
//                 )
//         )
    }
}

fn main() {
    ApplicationState::run(
        Settings {
            window: window::Settings {
                icon: None, // TODO
                ..window::Settings::default()
            },
            ..Settings::default()
        }
    ).unwrap();
}
