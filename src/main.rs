#![feature(const_fn_floating_point_arithmetic)]
#![feature(bindings_after_at)]
#![feature(iterator_fold_self)]
//!
//! Task list:
//! * Custom UI rendering:
//!     * Procedurally generated UI (Iced)
//!     * CPU Canvas (WASM) https://github.com/embedded-graphics/embedded-graphics
//!     * Node Definitions (displaying GPU-rendered texture)
//! * Display type tooltips when hovering over channels
//! * Add `NodeElement` styles
//!

use graph::*;
use iced::{window, Application, Command, Settings};
use iced_wgpu::Renderer;
use node::*;
use petgraph::graph::NodeIndex;
use petgraph::{stable_graph::StableGraph, Directed, Direction};
use std::borrow::Cow;
use std::collections::HashSet;
use style::*;
use widgets::*;

pub mod graph;
pub mod node;
pub mod style;
pub mod util;
pub mod widgets;

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

struct ApplicationState {
    graph: Graph,
    floating_panes_state: FloatingPanesState,
    floating_panes_content_state: FloatingPanesBehaviourState,
}

impl ApplicationState {
    pub fn is_graph_complete(&self) -> bool {
        for node_index in self.graph.node_indices() {
            let node = self.graph.node_weight(node_index);
            let node = node.as_ref().unwrap();
            let mut input_channels =
                (0..node.configuration.channels_input.len()).into_iter().collect::<HashSet<_>>();

            for edge in self.graph.edges_directed(node_index, Direction::Incoming) {
                input_channels.remove(&edge.weight().channel_index_to);
            }

            if !input_channels.is_empty() {
                return false;
            }
        }

        true
    }

    pub fn execute_graph(&mut self) {
        if !self.is_graph_complete() {
            return;
        }

        match petgraph::algo::toposort(&self.graph, None) {
            Ok(ordered_node_indices) => {
                for node_index in ordered_node_indices {
                    {
                        let mut node = self.graph.node_weight_mut(node_index);
                        let node = node.as_mut().unwrap();

                        node.ready_output_values();
                    }

                    let node = self.graph.node_weight(node_index);
                    let node = node.as_ref().unwrap();
                    // FIXME: Because StableGraph::find_edge returns an `Option<_>` rather than
                    // `Vec<_>`, we must iterate over all edges
                    let input_values = ChannelValues {
                        values: {
                            let mut input_values: Vec<Option<ChannelValue>> =
                                vec![None; node.configuration.channels_input.len()];
                            for edge_index in self.graph.edge_indices() {
                                let (from_index, to_index) = self.graph.edge_endpoints(edge_index).unwrap();

                                if to_index == node_index {
                                    let from = self.graph.node_weight(from_index).unwrap();
                                    let edge = self.graph.edge_weight(edge_index).unwrap();

                                    input_values[edge.channel_index_to] = Some(
                                        from.execution_output_values.as_ref().unwrap().borrow().values
                                            [edge.channel_index_from]
                                            .clone(),
                                    );
                                }
                            }

                            input_values
                                .into_iter()
                                .map(|value| value.expect("An input channel is missing a value."))
                                .collect::<Vec<_>>()
                                .into_boxed_slice()
                        },
                    };
                    let mut output_values = node.execution_output_values.as_ref().unwrap().borrow_mut();
                    node.behaviour.execute(&input_values, &mut output_values);
                }
            }
            Err(cycle) => {
                // FIXME
                panic!("Found a cycle in the graph.");
            }
        }
    }
}

impl Application for ApplicationState {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = (); // The data needed to initialize your Application.

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            Self {
                graph: {
                    use ChannelType::*;
                    use OpaqueChannelType::*;
                    use PrimitiveChannelType::*;
                    let mut graph = Graph::new();

                    // graph.add_node(NodeData::new(
                    //     "Node A",
                    //     [10.0, 10.0],
                    //     Box::new(TestNodeBehaviour {
                    //         name: "Behaviour A".to_string(),
                    //         channels_input: vec![Primitive(U8)],
                    //         channels_output: vec![Primitive(U8), Primitive(U32)],
                    //     }),
                    // ));
                    // graph.add_node(NodeData::new(
                    //     "Node B",
                    //     [110.0, 10.0],
                    //     Box::new(TestNodeBehaviour {
                    //         name: "Behaviour B".to_string(),
                    //         channels_input: vec![Primitive(U8), Primitive(U8), Primitive(U32)],
                    //         channels_output: vec![Primitive(U8)],
                    //     }),
                    // ));
                    // graph.add_node(NodeData::new(
                    //     "Node C",
                    //     [210.0, 10.0],
                    //     Box::new(TestNodeBehaviour {
                    //         name: "Behaviour C".to_string(),
                    //         channels_input: vec![
                    //             Primitive(U8),
                    //             Array(ArrayChannelType::new(Array(ArrayChannelType::new(F32, 4)), 8)),
                    //         ],
                    //         channels_output: vec![
                    //             Primitive(U8),
                    //             Opaque(Texture(TextureChannelType {})),
                    //             Array(ArrayChannelType::new(U8, 4)),
                    //         ],
                    //     }),
                    // ));

                    graph.add_node(NodeData::new(
                        "My Constant Node #1",
                        [210.0, 10.0],
                        Box::new(ConstantNodeBehaviour { value: 42 }),
                    ));

                    graph.add_node(NodeData::new(
                        "My Constant Node #2",
                        [10.0, 10.0],
                        Box::new(ConstantNodeBehaviour { value: 84 }),
                    ));

                    graph.add_node(NodeData::new(
                        "My Bin Op #1",
                        [410.0, 10.0],
                        Box::new(BinaryOpNodeBehaviour { op: BinaryOp::Add }),
                    ));

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
        // FIXME: Decouple from UI rendering loop
        self.execute_graph();

        let theme: Box<dyn Theme> = Box::new(style::Dark);
        let node_indices = self.graph.node_indices().collect::<Vec<_>>();
        // TODO: do not recompute every time `view` is called
        let Self { graph, floating_panes_content_state, .. } = self;
        let mut connections = Vec::with_capacity(graph.edge_count());

        connections.extend(graph.edge_indices().map(|edge_index| {
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
                connections,
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
