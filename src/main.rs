#![feature(const_fn_floating_point_arithmetic)]
use iced::{window, Application, Command, Settings};
use iced_wgpu::Renderer;
use petgraph::graph::NodeIndex;
use petgraph::{stable_graph::StableGraph, Directed};
///
/// Task list:
/// * Make channel connections more responsive
/// * Define channel types
///
use std::borrow::Cow;
use style::*;
use widgets::*;

pub mod style;
pub mod util;
pub mod widgets;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelIdentifier {
    pub node_index: NodeIndex<u32>,
    pub channel_direction: ChannelDirection,
    pub channel_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection(pub [(NodeIndex<u32>, usize); 2]);

impl Connection {
    pub fn try_from_identifiers([a, b]: [ChannelIdentifier; 2]) -> Option<Connection> {
        if a.channel_direction == b.channel_direction {
            None
        } else {
            Some(Self(if a.channel_direction == ChannelDirection::Out {
                [(a.node_index, a.channel_index), (b.node_index, b.channel_index)]
            } else {
                [(b.node_index, b.channel_index), (a.node_index, a.channel_index)]
            }))
        }
    }

    pub fn contains_channel(&self, channel: ChannelIdentifier) -> bool {
        let index = match channel.channel_direction {
            ChannelDirection::In => 1,
            ChannelDirection::Out => 0,
        };
        let current = &self.0[index];

        current.0 == channel.node_index && current.1 == channel.channel_index
    }

    pub fn channel(&self, direction: ChannelDirection) -> ChannelIdentifier {
        let index = match direction {
            ChannelDirection::In => 1,
            ChannelDirection::Out => 0,
        };
        ChannelIdentifier {
            node_index: self.0[index].0,
            channel_direction: direction,
            channel_index: self.0[index].1,
        }
    }

    pub fn to(&self) -> ChannelIdentifier {
        self.channel(ChannelDirection::In)
    }

    pub fn from(&self) -> ChannelIdentifier {
        self.channel(ChannelDirection::Out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDirection {
    In,
    Out,
}

impl ChannelDirection {
    pub fn inverse(self) -> Self {
        match self {
            ChannelDirection::In => ChannelDirection::Out,
            ChannelDirection::Out => ChannelDirection::In,
        }
    }
}

struct Channel {
    pub title: String,
    pub description: Option<String>,
}

impl<'a> From<&'a Channel> for ChannelSlice<'a> {
    fn from(other: &'a Channel) -> Self {
        Self { title: &other.title, description: other.description.as_ref().map(String::as_str) }
    }
}

impl Channel {
    pub fn new(title: impl ToString) -> Self {
        Self { title: title.to_string(), description: None }
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
    floating_pane_content_state: FloatingPaneContentState,
    input_channels: Vec<Channel>,
    output_channels: Vec<Channel>,
}

impl NodeData {
    pub fn view(
        &mut self,
        index: NodeIndex<u32>,
        theme: &dyn Theme,
    ) -> FloatingPane<'_, Message, Renderer, NodeElement<'_, Message, Renderer>>
    {
        let mut builder = NodeElement::builder(index, &mut self.element_state);

        for input_channel in &self.input_channels {
            builder = builder.push_input_channel(input_channel);
        }

        for output_channel in &self.output_channels {
            builder = builder.push_output_channel(output_channel);
        }

        let node_element = builder.build(/*|index, new_value| {
            Message::NodeMessage {
                node: index,
                message: NodeMessage::UpdateTextInput(new_value),
            }
        }*/);

        FloatingPane::builder(
            node_element,
            &mut self.floating_pane_state,
            &mut self.floating_pane_content_state,
        )
        .title(Some(&self.title))
        .title_size(Some(16))
        .title_margin(consts::SPACING)
        .style(Some(theme.floating_pane()))
        .build()
    }
}

struct EdgeData {
    channel_index_from: usize,
    channel_index_to: usize,
}

impl EdgeData {
    fn get_channel_index(&self, direction: ChannelDirection) -> usize {
        match direction {
            ChannelDirection::In => self.channel_index_from,
            ChannelDirection::Out => self.channel_index_to,
        }
    }
}

type Graph = StableGraph<
    NodeData, // Node Data
    EdgeData, // Edge Data
    Directed, // Edge Type
    u32,      // Node Index
>;

struct ApplicationState {
    graph: Graph,
    floating_panes_state: FloatingPanesState,
    floating_panes_content_state: FloatingPanesContentState,
}

#[derive(Debug, Clone)]
pub enum NodeMessage {
    UpdateTextInput(String),
}

#[derive(Debug, Clone)]
pub enum Message {
    NodeMessage { node: NodeIndex<u32>, message: NodeMessage },
    DisconnectChannel { channel: ChannelIdentifier },
    InsertConnection { connection: Connection },
}

impl Application for ApplicationState {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = (); // The data needed to initialize your Application.

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            Self {
                graph: {
                    let mut graph = Graph::new();

                    let node_a = graph.add_node(NodeData {
                        title: "Node A".to_string(),
                        element_state: Default::default(),
                        floating_pane_state: FloatingPaneState::with_position([10.0, 10.0]),
                        floating_pane_content_state: Default::default(),
                        input_channels: vec![Channel::new("In A")],
                        output_channels: vec![Channel::new("Out A"), Channel::new("Out B")],
                    });

                    let node_b = graph.add_node(NodeData {
                        title: "Node B".to_string(),
                        element_state: Default::default(),
                        floating_pane_state: FloatingPaneState::with_position([100.0, 10.0]),
                        floating_pane_content_state: Default::default(),
                        input_channels: vec![
                            Channel::new("In A"),
                            Channel::new("In B"),
                            Channel::new("In C"),
                        ],
                        output_channels: vec![Channel::new("Out A")],
                    });

                    let node_indices: Vec<_> = graph.node_indices().collect();
                    for (node_index, node) in node_indices.iter().zip(graph.node_weights_mut()) {
                        node.floating_pane_content_state.node_index = Some(*node_index);
                    }

                    graph.add_edge(node_a, node_b, EdgeData { channel_index_from: 0, channel_index_to: 2 });

                    graph.add_edge(node_a, node_b, EdgeData { channel_index_from: 1, channel_index_to: 0 });

                    graph
                },
                floating_panes_state: Default::default(),
                floating_panes_content_state: FloatingPanesContentState::default(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("DVSynth")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::NodeMessage { node, message } => {
                match message {
                    NodeMessage::UpdateTextInput(new_value) => {
                        if let Some(node_data) = self.graph.node_weight_mut(node) {
                            // node_data.element_state.text_input_value = new_value;
                        }
                    }
                }
            }
            Message::DisconnectChannel { channel } => {
                self.graph.retain_edges(|frozen, edge| {
                    let (from, to) = frozen.edge_endpoints(edge).unwrap();
                    let node_index = match channel.channel_direction {
                        ChannelDirection::In => to,
                        ChannelDirection::Out => from,
                    };

                    if node_index == channel.node_index {
                        let edge_data = frozen.edge_weight(edge).unwrap();

                        if edge_data.get_channel_index(channel.channel_direction.inverse())
                            == channel.channel_index
                        {
                            return false;
                        }
                    }

                    true
                });
            }
            Message::InsertConnection { connection } => {
                let from = connection.from();
                let to = connection.to();

                self.graph.add_edge(
                    from.node_index,
                    to.node_index,
                    EdgeData { channel_index_from: from.channel_index, channel_index_to: to.channel_index },
                );
            }
        }

        Command::none()
    }

    fn view(&mut self) -> iced::Element<Message> {
        let theme: Box<dyn Theme> = Box::new(style::Dark);
        let node_indices = self.graph.node_indices().collect::<Vec<_>>();
        // TODO: do not recompute every time `view` is called
        let Self { graph, floating_panes_content_state, .. } = self;
        floating_panes_content_state.connections.clear();
        floating_panes_content_state.connections.extend(graph.edge_indices().map(|edge_index| {
            let edge_data = &graph[edge_index];
            let (index_from, index_to) = graph.edge_endpoints(edge_index).unwrap();
            Connection([(index_from, edge_data.channel_index_from), (index_to, edge_data.channel_index_to)])
        }));

        let mut panes =
            FloatingPanes::new(&mut self.floating_panes_state, &mut self.floating_panes_content_state)
                .style(theme.floating_panes());

        for (node_index, node_data) in node_indices.iter().zip(self.graph.node_weights_mut()) {
            panes = panes.insert(*node_index, node_data.view(*node_index, theme.as_ref()));
        }

        panes.into()
    }
}

fn main() {
    ApplicationState::run(Settings {
        window: window::Settings {
            icon: None, // TODO
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Settings::default()
    })
    .unwrap();
}
