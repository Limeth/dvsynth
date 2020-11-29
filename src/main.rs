#![feature(generic_associated_types)]
#![feature(negative_impls)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(bindings_after_at)]
#![feature(iterator_fold_self)]
#![feature(trivial_bounds)]
#![feature(associated_type_defaults)]
#![feature(trait_alias)]
//!
//! Task list:
//! * Window node:
//!     * Make window size accessible only when resizable is false
//!     * Fullscreen modes
//! * Use `libloading` to load node implementations as cdylibs.
//! * Mark invalid connections and cycles in the graph
//! * Custom UI rendering:
//!     * CPU Canvas (WASM) https://github.com/embedded-graphics/embedded-graphics
//!     * Node Definitions (displaying GPU-rendered texture)
//! * Display type tooltips when hovering over channels
//!

use graph::{
    ApplicationContext, ChannelIdentifier, Connection, EdgeData, ExecutionGraph, Graph, GraphExecutor,
    NodeData,
};
use iced::{window, Application, Command, Settings};
use iced_winit::winit;
use node::behaviour::*;
use node::*;
use petgraph::graph::NodeIndex;
use style::Themeable;
use style::*;
use widgets::*;
pub mod graph;
pub mod node;
pub mod style;
pub mod util;
pub mod widgets;

#[derive(Debug, Clone)]
pub enum NodeMessage {
    NodeBehaviourMessage(Box<dyn NodeBehaviourMessage>),
}

#[derive(Debug, Clone)]
pub enum Message {
    NodeMessage {
        node: NodeIndex<u32>,
        message: NodeMessage,
    },
    DisconnectChannel {
        channel: ChannelIdentifier,
    },
    InsertConnection {
        connection: Connection,
    },
    /// Workaround for layouts not being updated when we only change its mutable state
    RecomputeLayout,
}

pub struct ApplicationFlags {
    graph: ExecutionGraph,
}

pub struct ApplicationState {
    graph: ExecutionGraph,
    floating_panes_state: FloatingPanesState,
    floating_panes_content_state: FloatingPanesBehaviourState,
}

impl Application for ApplicationState {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ApplicationFlags; // The data needed to initialize your Application.

    fn new(flags: ApplicationFlags) -> (Self, Command<Self::Message>) {
        (
            Self {
                graph: flags.graph,
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
        let mut update_schedule = false;

        match message {
            Message::NodeMessage { node, message } => {
                match message {
                    NodeMessage::NodeBehaviourMessage(message) => {
                        if let Some(node_data) = self.graph.node_weight_mut(node) {
                            node_data.update(NodeEvent::Message(message));
                        }
                    }
                }

                update_schedule = true;
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

                update_schedule = true;
            }
            Message::InsertConnection { connection } => {
                let from = connection.from();
                let to = connection.to();

                self.graph.add_edge(
                    from.node_index,
                    to.node_index,
                    EdgeData { channel_index_from: from.channel_index, channel_index_to: to.channel_index },
                );

                update_schedule = true;
            }
            Message::RecomputeLayout => (),
        }

        if update_schedule {
            if let Err(_) = self.graph.update_schedule() {
                eprintln!("Could not construct the graph schedule.");
            }
        }

        Command::none()
    }

    fn view(&mut self) -> iced::Element<Message> {
        let theme: Box<dyn Theme> = Box::new(style::Dark);
        let node_indices = self.graph.node_indices().collect::<Vec<_>>();
        let mut connections = Vec::with_capacity(self.graph.edge_count());

        connections.extend(self.graph.edge_indices().map(|edge_index| {
            let edge_data = &self.graph[edge_index];
            let (index_from, index_to) = self.graph.edge_endpoints(edge_index).unwrap();
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
            Box::new(|| Message::RecomputeLayout),
        )
        .theme(&*theme);

        for (node_index, node_data) in node_indices.iter().zip(self.graph.node_weights_mut()) {
            panes = panes.insert(*node_index, node_data.view(*node_index, theme.as_ref()));
        }

        panes.into()
    }
}

fn main() {
    let graph: ExecutionGraph = {
        let mut graph = Graph::new();

        graph.add_node(NodeData::new(
            "My Constant Node #1",
            [210.0, 10.0],
            Box::new(ConstantNodeBehaviour::new(42.0_f32)),
        ));

        graph.add_node(NodeData::new(
            "My Constant Node #2",
            [10.0, 10.0],
            Box::new(ConstantNodeBehaviour::new(84.0_f32)),
        ));

        // graph.add_node(NodeData::new(
        //     "My Bin Op #1",
        //     [410.0, 10.0],
        //     Box::new(BinaryOpNodeBehaviour::default()),
        // ));

        // graph.add_node(NodeData::new(
        //     "My Window #1",
        //     [610.0, 10.0],
        //     Box::new(WindowNodeBehaviour::default()),
        // ));

        // graph.add_node(NodeData::new(
        //     "My Array Constructor",
        //     [10.0, 510.0],
        //     Box::new(ArrayConstructorNodeBehaviour::default()),
        // ));

        graph.add_node(NodeData::new(
            "My List Constructor",
            [10.0, 710.0],
            Box::new(ListConstructorNodeBehaviour::default()),
        ));

        graph.add_node(NodeData::new("My Debug", [210.0, 510.0], Box::new(DebugNodeBehaviour::default())));

        graph.add_node(NodeData::new("My Counter", [810.0, 10.0], Box::new(CounterNodeBehaviour::default())));

        graph.into()
    };

    let active_schedule = graph.active_schedule.clone();
    let settings = Settings {
        window: window::Settings {
            icon: None, // TODO
            ..window::Settings::default()
        },
        antialiasing: true,
        ..Settings::with_flags(ApplicationFlags { graph })
    };
    let (execution_context, main_thread_task_receiver) = ApplicationContext::from_settings(&settings);
    let renderer_settings = iced_wgpu::Settings {
        default_font: settings.default_font,
        default_text_size: settings.default_text_size,
        // because anti-aliasing is enabled in the settings
        antialiasing: Some(iced_wgpu::Antialiasing::MSAAx4),
        instance: Some(execution_context.renderer.instance.clone()),
        device_queue: Some((
            execution_context.renderer.device.clone(),
            execution_context.renderer.queue.clone(),
        )),
        ..iced_wgpu::Settings::default()
    };
    let _join_handle = GraphExecutor::spawn(execution_context, active_schedule);

    ApplicationState::run_with_event_handler_and_renderer_settings(
        settings,
        renderer_settings,
        Some(Box::new(move |event, window_target, _control_flow| {
            if event == winit::event::Event::MainEventsCleared {
                for main_thread_task in main_thread_task_receiver.try_iter() {
                    (main_thread_task)(window_target);
                }
            }
        })),
    )
    .unwrap();
}
