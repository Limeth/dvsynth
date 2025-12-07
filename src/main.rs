#![feature(array_windows)]
#![feature(never_type)]
#![feature(ptr_metadata)]
#![feature(negative_impls)]
#![feature(trivial_bounds)]
#![feature(associated_type_defaults)]
#![feature(trait_alias)]
#![feature(if_let_guard)]
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

use std::marker::PhantomData;
use std::{any::TypeId, collections::BTreeMap};

use graph::{
    ChannelIdentifier, Connection, EdgeData, ExecutionGraph, Graph, GraphValidationErrors, NodeData,
};
use iced::{Settings, Task, Theme, window};
use indexmap::IndexMap;
use node::behaviour::counter::CounterNodeBehaviour;
use node::behaviour::*;
use node::*;
use petgraph::graph::NodeIndex;
use tokio::runtime::Runtime;
use tracing::trace_span;
use vek::Vec2;
use views::{FloatingPaneExt, FloatingPaneParams, floating_panes};
use xilem::dpi::Position;
use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{AccessCtx, BoxConstraints, ComposeCtx, LayoutCtx, PaintCtx, PropertiesRef};
use xilem::masonry::kurbo::Size;
use xilem::masonry::peniko::color::{AlphaColor, Srgb};
use xilem::masonry::vello::Scene;
use xilem::style::Background;
use xilem::view::{FlexExt, FlexParams, flex_item};
use xilem::{
    EventLoop, WidgetView, WindowOptions, Xilem,
    core::View,
    masonry::{
        core::{
            AccessEvent, ChildrenIds, EventCtx, NoAction, PointerEvent, PropertiesMut, QueryCtx, RegisterCtx,
            TextEvent, Update, UpdateCtx, Widget, WidgetId, WidgetPod,
        },
        kurbo::Point,
    },
    view::{Axis, flex, label, text_button},
    winit::window::CursorIcon,
};
use xilem::{Pod, ViewCtx};

#[macro_use]
pub mod util;

pub mod graph;
pub mod node;
pub mod style;
pub mod views;
pub mod widgets;

type Message = ();
type Element<'a, M, T = Theme> = iced::Element<'a, M, T, iced_wgpu::Renderer>;

pub struct ApplicationFlags {
    graph: ExecutionGraph,
}

pub struct ApplicationState {
    graph: ExecutionGraph,
    graph_validation_errors: GraphValidationErrors,
}

impl ApplicationState {
    // type Executor = iced::executor::Default;
    // type Message = Message;
    // type Flags = ApplicationFlags; // The data needed to initialize your Application.

    // fn new(flags: ApplicationFlags) -> (Self, Task<Message>) {
    //     let (_window_id, task_window_open) = window::open(window::Settings::default());

    //     (
    //         Self {
    //             theme: Theme::Dark,
    //             graph: flags.graph,
    //             floating_panes_state: Default::default(),
    //             floating_panes_content_state: FloatingPanesBehaviourState::default(),
    //             graph_validation_errors: Default::default(),
    //             windows: Default::default(),
    //         },
    //         task_window_open.map(Message::WindowOpened),
    //     )
    // }

    // fn title(&self, window_id: window::Id) -> String {
    //     format!("DVSynth Window #{window_id}")
    // }

    // fn theme(&self, _window_id: window::Id) -> Theme {
    //     self.theme.clone()
    // }

    // fn update(&mut self, message: Message) -> Task<Message> {
    //     let mut update_schedule = false;
    //     let task = match message {
    //         Message::WindowOpened(id) => {
    //             let window = WindowHandle {};
    //             // Focus an element:
    //             // let focus_input = text_input::focus(format!("input-{id}"));

    //             self.windows.insert(id, window);

    //             // focus_input
    //             Task::none()
    //         }
    //         Message::NodeMessage { node, message } => {
    //             match message {
    //                 NodeMessage::NodeBehaviourMessage(message) => {
    //                     if let Some(node_data) = self.graph.node_weight_mut(node) {
    //                         node_data.update(NodeEvent::Message(message));
    //                     }
    //                 }
    //             }

    //             update_schedule = true;
    //             Task::none()
    //         }
    //         Message::DisconnectChannel { channel } => {
    //             self.graph.retain_edges(|frozen, edge| {
    //                 let (from, to) = frozen.edge_endpoints(edge).unwrap();
    //                 let node_index = match channel.channel_direction {
    //                     ChannelDirection::In => to,
    //                     ChannelDirection::Out => from,
    //                 };

    //                 if node_index == channel.node_index {
    //                     let edge_data = frozen.edge_weight(edge).unwrap();

    //                     if edge_data.get_endpoint(channel.channel_direction.inverse()) == channel.into() {
    //                         return false;
    //                     }
    //                 }

    //                 true
    //             });

    //             update_schedule = true;
    //             Task::none()
    //         }
    //         Message::InsertConnection { connection } => {
    //             let from = connection.from();
    //             let to = connection.to();

    //             self.graph.add_edge(
    //                 from.node_index,
    //                 to.node_index,
    //                 EdgeData { endpoint_from: from.into(), endpoint_to: to.into() },
    //             );

    //             update_schedule = true;
    //             Task::none()
    //         }
    //         Message::FloatingPanesPaneMoveTo { pane_index, position } => {
    //             if let Some(pane) = self.graph.node_weight_mut(pane_index) {
    //                 pane.floating_pane_state.position = position;
    //             }
    //             Task::none()
    //         }
    //         Message::FloatingPanesPaneResize { pane_index, size } => {
    //             if let Some(pane) = self.graph.node_weight_mut(pane_index) {
    //                 pane.floating_pane_state.size = size;
    //             }
    //             Task::none()
    //         }
    //         Message::FloatingPanesBackgroundMoveTo { position } => {
    //             self.floating_panes_state.panes_offset = position;
    //             Task::none()
    //         }
    //         Message::FloatingPanesGestureChange { gesture } => {
    //             self.floating_panes_state.gesture = gesture;
    //             Task::none()
    //         }
    //         Message::FloatingPanesPaneTitleBarStatusChange { pane_index, title_bar_status } => {
    //             if let Some(pane) = self.graph.node_weight_mut(pane_index) {
    //                 pane.floating_pane_state.title_bar_status = title_bar_status;
    //             }
    //             Task::none()
    //         }
    //         Message::FloatingPanesHighlightChanged { highlight } => {
    //             self.floating_panes_content_state.highlight = highlight;
    //             Task::none()
    //         }
    //         Message::FloatingPanesSelectedChannelChanged { selected_channel } => {
    //             self.floating_panes_content_state.selected_channel = selected_channel;
    //             Task::none()
    //         }
    //         Message::RecomputeLayout => Task::none(),
    //     };

    //     if update_schedule {
    //         if let Err(vec) = self.graph.update_schedule() {
    //             eprintln!("Could not construct the graph schedule:\n{:?}", vec);
    //             self.graph_validation_errors = vec.into();
    //         } else {
    //             self.graph_validation_errors = Default::default();
    //         }
    //     }

    //     task
    // }

    // fn view_old(&self, window_id: window::Id) -> Element<'_> {
    //     let theme = Theme::Dark; // TODO: Theming
    //     let node_indices = self.graph.node_indices().collect::<Vec<_>>();
    //     let connections = self.graph.get_connections();

    //     // TODO: Pass const references, and send messages back instead of trying to mutate state through
    //     // the references.

    //     let mut panes = FloatingPanes::new(
    //         &self.floating_panes_state,
    //         &self.floating_panes_content_state,
    //         crate::widgets::node::FloatingPanesBehaviour {
    //             on_channel_disconnect: |channel| Message::DisconnectChannel { channel },
    //             on_connection_create: |connection| Message::InsertConnection { connection },
    //             on_highlight_change: |highlight| Message::FloatingPanesHighlightChanged { highlight },
    //             on_selected_channel_changed: |selected_channel| {
    //                 Message::FloatingPanesSelectedChannelChanged { selected_channel }
    //             },
    //             connections,
    //             graph_validation_errors: self.graph_validation_errors.clone(),
    //             // tooltip_style: Some(theme.tooltip()),
    //             __marker: PhantomData,
    //         },
    //         Box::new(|| Message::RecomputeLayout),
    //         Box::new(|pane_index, position| Message::FloatingPanesPaneMoveTo { pane_index, position }),
    //         Box::new(|pane_index, size| Message::FloatingPanesPaneResize { pane_index, size }),
    //         Box::new(|position| Message::FloatingPanesBackgroundMoveTo { position }),
    //         Box::new(|gesture| Message::FloatingPanesGestureChange { gesture }),
    //         Box::new(|pane_index, title_bar_status| Message::FloatingPanesPaneTitleBarStatusChange {
    //             pane_index,
    //             title_bar_status,
    //         }),
    //     );
    //     // .theme(&*theme);

    //     for (node_index, node_data) in node_indices.iter().zip(self.graph.node_weights()) {
    //         panes = panes.insert(*node_index, node_data.view(*node_index, &theme));
    //     }

    //     panes.into()
    // }

    fn view(&mut self) -> impl WidgetView<ApplicationState> + use<> {
        flex(
            Axis::Vertical,
            (
                label(format!("{}", self.graph.node_count())),
                text_button("increment", |state: &mut ApplicationState| state.graph.clear()),
                label("Flex").flex(FlexParams::new(Some(100.0), None)),
                floating_panes((
                    label("Foo").floating_pane(
                        (),
                        (),
                        FloatingPaneParams {
                            title: "Foo's Title".into(),
                            position_local: (0.0, 100.0).into(),
                        },
                    ),
                    flex(Axis::Vertical, (label("Bar"), text_button("press me", |_| println!("yeet"))))
                        .floating_pane(
                            (label("in 1"),),
                            (label("out 1"), text_button("out 2", |_| println!("yeet"))),
                            FloatingPaneParams {
                                title: "Bar's Title".into(),
                                position_local: (100.0, 0.0).into(),
                            },
                        ),
                ))
                .prop(Background::Color(AlphaColor::from_rgb8(0x1F, 0x1F, 0x1F))),
            ),
        )
    }
}

#[tokio::main]
async fn main() {
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

        // graph.add_node(NodeData::new(
        //     "My Window #1",
        //     [610.0, 10.0],
        //     Box::new(WindowNodeBehaviour::default()),
        // ));

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

        graph.add_node(NodeData::new("My Counter", [810.0, 10.0], Box::new(CounterNodeBehaviour)));

        graph.into()
    };

    let active_schedule = graph.active_schedule.clone();
    let settings = Settings {
        // window: window::Settings {
        //     icon: None, // TODO
        //     ..window::Settings::default()
        // },
        antialiasing: true,
        ..Default::default() // ..Settings::with_flags(ApplicationFlags { graph })
    };
    // let event_loop = EventLoop::new().expect("Failed to create event loop.");
    // let (execution_context, main_thread_task_receiver) =
    //     ApplicationContext::from_settings(&settings, window.clone()).await;
    // let renderer_settings = iced_wgpu::Settings {
    //     default_font: settings.default_font,
    //     default_text_size: settings.default_text_size,
    //     // because anti-aliasing is enabled in the settings
    //     antialiasing: Some(iced_graphics::Antialiasing::MSAAx4),
    //     ..iced_wgpu::Settings::default()
    // };
    // let _join_handle = GraphExecutor::spawn(execution_context, active_schedule);

    // let mut engine = Engine::new(
    //     &execution_context.renderer.adapter,
    //     &execution_context.renderer.device,
    //     &execution_context.renderer.queue,
    //     execution_context.renderer.surface_format,
    //     Some(Antialiasing::MSAAx16),
    // );
    // let mut iced_renderer = iced_wgpu::Renderer::new(
    //     Runtime::new(
    //         &execution_context.renderer.device,
    //         &execution_context.renderer.queue,
    //         // iced_wgpu::Settings::default(),
    //         // execution_context.renderer.surface_format,
    //     ),
    //     &engine,
    //     Font::DEFAULT,
    //     Pixels(16.0),
    // );

    let app = Xilem::new_simple(
        ApplicationState { graph, graph_validation_errors: Default::default() },
        ApplicationState::view,
        WindowOptions::new("dvsynth"),
    );
    app.run_in(EventLoop::with_user_event()).unwrap();

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
