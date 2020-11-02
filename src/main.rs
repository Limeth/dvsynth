#![feature(const_fn_floating_point_arithmetic)]
#![feature(bindings_after_at)]
#![feature(iterator_fold_self)]
//!
//! Task list:
//! * Use acceleration structures to check for incidence/highlights with connection points and
//!   connections
//! * Define channel types
//! * Add `NodeElement` styles
//!

use graph::*;
use iced::{window, Application, Command, Settings};
use iced_wgpu::Renderer;
use node::*;
use petgraph::graph::NodeIndex;
use petgraph::{stable_graph::StableGraph, Directed};
use std::borrow::Cow;
use style::*;
use widgets::*;

pub mod graph;
pub mod node;
pub mod style;
pub mod util;
pub mod widgets;

struct ApplicationState {
    graph: Graph,
    floating_panes_state: FloatingPanesState,
    floating_panes_content_state: FloatingPanesBehaviourState,
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

                    let node_c = graph.add_node(NodeData {
                        title: "Node C".to_string(),
                        element_state: Default::default(),
                        floating_pane_state: FloatingPaneState::with_position([200.0, 10.0]),
                        floating_pane_content_state: Default::default(),
                        input_channels: vec![Channel::new("In A"), Channel::new("In B")],
                        output_channels: vec![
                            Channel::new("Out A"),
                            Channel::new("Out B"),
                            Channel::new("Out C"),
                        ],
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
                floating_panes_content_state: FloatingPanesBehaviourState::default(),
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

        let mut panes = FloatingPanes::new(
            &mut self.floating_panes_state,
            &mut self.floating_panes_content_state,
            crate::widgets::node::FloatingPanesBehaviour {
                on_channel_disconnect: |channel| Message::DisconnectChannel { channel },
                on_connection_create: |connection| Message::InsertConnection { connection },
            },
        )
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
