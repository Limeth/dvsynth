#![feature(array_windows)]
#![feature(associated_type_bounds)]
#![feature(never_type)]
#![feature(ptr_metadata)]
#![feature(negative_impls)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(trivial_bounds)]
#![feature(associated_type_defaults)]
#![feature(trait_alias)]
//!
//! Task list:
//! * Finish adding generic params to channel types
//! * Explore the usage of associated types to avoid dynamic allocation for
//!   owned references of sized channel types.
//! * Node generics
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
    GraphValidationErrors, NodeData,
};
use iced::application::Title;
use iced::{window, Application, Font, Pixels, Settings, Task};
use iced_futures::Runtime;
use iced_graphics::Antialiasing;
use iced_wgpu::Engine;
use iced_winit::winit;
use iced_winit::winit::event_loop::EventLoop;
use iced_winit::winit::window::Window;
use node::behaviour::counter::CounterNodeBehaviour;
use node::behaviour::*;
use node::*;
use petgraph::graph::NodeIndex;
use widgets::*;

#[macro_use]
pub mod util;

pub mod graph;
pub mod node;
pub mod style;
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
    graph_validation_errors: GraphValidationErrors,
}

impl ApplicationState {
    // type Executor = iced::executor::Default;
    // type Message = Message;
    // type Flags = ApplicationFlags; // The data needed to initialize your Application.

    fn new(flags: ApplicationFlags) -> (Self, Task<Self::Message>) {
        (
            Self {
                graph: flags.graph,
                floating_panes_state: Default::default(),
                floating_panes_content_state: FloatingPanesBehaviourState::default(),
                graph_validation_errors: Default::default(),
            },
            Task::none(),
        )
    }

    fn title(&self) -> String {
        String::from("DVSynth")
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
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

                        if edge_data.get_endpoint(channel.channel_direction.inverse()) == channel.into() {
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
                    EdgeData { endpoint_from: from.into(), endpoint_to: to.into() },
                );

                update_schedule = true;
            }
            Message::RecomputeLayout => (),
        }

        if update_schedule {
            if let Err(vec) = self.graph.update_schedule() {
                eprintln!("Could not construct the graph schedule:\n{:?}", vec);
                self.graph_validation_errors = vec.into();
            } else {
                self.graph_validation_errors = Default::default();
            }
        }

        Task::none()
    }

    fn view(&mut self) -> iced::Element<Message> {
        // let theme: Box<dyn Theme> = Box::new(style::Dark);
        let node_indices = self.graph.node_indices().collect::<Vec<_>>();
        let connections = self.graph.get_connections();

        let mut panes = FloatingPanes::new(
            &mut self.floating_panes_state,
            &mut self.floating_panes_content_state,
            crate::widgets::node::FloatingPanesBehaviour {
                on_channel_disconnect: |channel| Message::DisconnectChannel { channel },
                on_connection_create: |connection| Message::InsertConnection { connection },
                connections,
                graph_validation_errors: self.graph_validation_errors.clone(),
                tooltip_style: Some(theme.tooltip()),
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

        graph.add_node(NodeData::new(
            "My Bin Op #1",
            [410.0, 10.0],
            Box::new(BinaryOpNodeBehaviour::default()),
        ));

        graph.add_node(NodeData::new(
            "My Window #1",
            [610.0, 10.0],
            Box::new(WindowNodeBehaviour::default()),
        ));

        graph.add_node(NodeData::new(
            "My Array Constructor",
            [10.0, 310.0],
            Box::new(ArrayConstructorNodeBehaviour::default()),
        ));

        graph.add_node(NodeData::new(
            "My List Constructor",
            [10.0, 510.0],
            Box::new(ListConstructorNodeBehaviour::default()),
        ));

        graph.add_node(NodeData::new("My Debug", [210.0, 510.0], Box::new(DebugNodeBehaviour::default())));
        graph.add_node(NodeData::new("My Debug 2", [410.0, 510.0], Box::new(DebugNodeBehaviour::default())));

        graph.add_node(NodeData::new("My Counter", [810.0, 10.0], Box::new(CounterNodeBehaviour::default())));

        graph.into()
    };

    let active_schedule = graph.active_schedule.clone();
    let settings = Settings {
        // window: window::Settings {
        //     icon: None, // TODO
        //     ..window::Settings::default()
        // },
        antialiasing: true,
        ..Settings::with_flags(ApplicationFlags { graph })
    };
    let event_loop = EventLoop::new().expect("Failed to create event loop.");
    let window = Window::new(&event_loop);
    let (execution_context, main_thread_task_receiver) =
        ApplicationContext::from_settings(&settings, window.clone());
    let renderer_settings = iced_wgpu::Settings {
        default_font: settings.default_font,
        default_text_size: settings.default_text_size,
        // because anti-aliasing is enabled in the settings
        antialiasing: Some(iced_graphics::Antialiasing::MSAAx4),
        ..iced_wgpu::Settings::default()
    };
    let _join_handle = GraphExecutor::spawn(execution_context, active_schedule);

    let mut engine = Engine::new(
        &execution_context.renderer.adapter,
        &execution_context.renderer.device,
        &execution_context.renderer.queue,
        execution_context.renderer.surface_format,
        Some(Antialiasing::MSAAx16),
    );
    let mut iced_renderer = iced_wgpu::Renderer::new(
        Runtime::new(
            &execution_context.renderer.device,
            &execution_context.renderer.queue,
            // iced_wgpu::Settings::default(),
            // execution_context.renderer.surface_format,
        ),
        &engine,
        Font::DEFAULT,
        Pixels(16.0),
    );

    iced::application(ApplicationState::title, ApplicationState::update, ApplicationState::view)
        .subscription(ApplicationState::subscription)
        .theme(ApplicationState::theme)
        .run_with(|(x, y)| ApplicationState::new(ApplicationFlags { graph }));

    // Main loop
    /*
    {
        let mut resized = false;

        event_loop
            .run(move |event, window_target| {
                // You should change this if you want to render continuosly
                window_target.set_control_flow(ControlFlow::Wait);

                match event {
                    Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
                        if resized {
                            let size = window.inner_size();

                            viewport = Viewport::with_physical_size(
                                Size::new(size.width, size.height),
                                window.scale_factor(),
                            );

                            surface.configure(
                                &device,
                                &wgpu::SurfaceConfiguration {
                                    format,
                                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                    width: size.width,
                                    height: size.height,
                                    present_mode: wgpu::PresentMode::AutoVsync,
                                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                                    view_formats: vec![],
                                    desired_maximum_frame_latency: 2,
                                },
                            );

                            resized = false;
                        }

                        match surface.get_current_texture() {
                            Ok(frame) => {
                                let mut encoder = device
                                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                                let program = state.program();

                                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                                {
                                    // We clear the frame
                                    let mut render_pass =
                                        Scene::clear(&view, &mut encoder, program.background_color());

                                    // Draw the scene
                                    scene.draw(&mut render_pass);
                                }

                                // And then iced on top
                                renderer.with_primitives(|backend, primitive| {
                                    backend.present(
                                        &device,
                                        &queue,
                                        &mut encoder,
                                        None,
                                        frame.texture.format(),
                                        &view,
                                        primitive,
                                        &viewport,
                                        &debug.overlay(),
                                    );
                                });

                                // Then we submit the work
                                queue.submit(Some(encoder.finish()));
                                frame.present();

                                // Update the mouse cursor
                                window.set_cursor_icon(iced_winit::conversion::mouse_interaction(
                                    state.mouse_interaction(),
                                ));
                            }
                            Err(error) => match error {
                                wgpu::SurfaceError::OutOfMemory => {
                                    panic!(
                                        "Swapchain error: {error}. \
                                Rendering cannot continue."
                                    )
                                }
                                _ => {
                                    // Try rendering again next frame.
                                    window.request_redraw();
                                }
                            },
                        }
                    }
                    Event::WindowEvent { event, .. } => {
                        match event {
                            WindowEvent::CursorMoved { position, .. } => {
                                cursor_position = Some(position);
                            }
                            WindowEvent::ModifiersChanged(new_modifiers) => {
                                modifiers = new_modifiers.state();
                            }
                            WindowEvent::Resized(_) => {
                                resized = true;
                            }
                            WindowEvent::CloseRequested => {
                                window_target.exit();
                            }
                            _ => {}
                        }

                        // Map window event to iced event
                        if let Some(event) = iced_winit::conversion::window_event(
                            window::Id::MAIN,
                            event,
                            window.scale_factor(),
                            modifiers,
                        ) {
                            state.queue_event(event);
                        }
                    }
                    _ => {}
                }

                // If there are events pending
                if !state.is_queue_empty() {
                    // We update iced
                    let _ = state.update(
                        viewport.logical_size(),
                        cursor_position
                            .map(|p| conversion::cursor_position(p, viewport.scale_factor()))
                            .map(mouse::Cursor::Available)
                            .unwrap_or(mouse::Cursor::Unavailable),
                        &mut renderer,
                        &Theme::Dark,
                        &renderer::Style { text_color: Color::WHITE },
                        &mut clipboard,
                        &mut debug,
                    );

                    // and request a redraw
                    window.request_redraw();
                }
            })
            .unwrap();
    } */

    // ApplicationState::run_with_event_handler_and_renderer_settings(
    //     settings,
    //     renderer_settings,
    //     Some(Box::new(move |event, window_target, _control_flow| {
    //         if event == winit::event::Event::MainEventsCleared {
    //             for main_thread_task in main_thread_task_receiver.try_iter() {
    //                 (main_thread_task)(window_target);
    //             }
    //         }
    //     })),
    // )
    // .unwrap();
}
