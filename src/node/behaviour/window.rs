use crate::graph::ApplicationContext;
use crate::{
    node::{
        behaviour::{ExecutionContext, NodeBehaviour, NodeCommand, NodeEvent},
        NodeConfiguration,
    },
    style::{Theme, Themeable},
};
use flume::{self, Receiver};
use iced::widget::checkbox::Checkbox;
use iced::widget::text_input::{self, TextInput};
use iced::{Column, Element, Row};
use iced_wgpu::wgpu;
use iced_winit::winit;
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;
use vek::Vec2;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Fullscreen, Window, WindowBuilder};

#[derive(Clone)]
pub enum WindowMessage {
    ModifyWindowSettings(Arc<dyn Fn(&mut WindowNodeBehaviour) + Send + Sync>),
}

impl Debug for WindowMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use WindowMessage::*;
        match self {
            ModifyWindowSettings(_) => write!(f, "ModifyWindowSettings"),
        }
    }
}

impl_node_behaviour_message!(WindowMessage);

#[derive(Debug, Clone)]
pub struct WindowSettings {
    title: Cow<'static, str>,
    inner_size: Vec2<u32>,
    fullscreen: Option<Fullscreen>,
    always_on_top: bool,
    cursor_grab: bool,
    cursor_visible: bool,
    decorations: bool,
    maximized: bool,
    minimized: bool,
    resizable: bool,
    visible: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            title: Cow::Borrowed("DVSynth Output Window"),
            inner_size: Vec2::new(800, 450),
            fullscreen: None,
            always_on_top: false,
            cursor_grab: false,
            cursor_visible: true,
            decorations: true,
            maximized: false,
            minimized: false,
            resizable: true,
            visible: true,
        }
    }
}

impl WindowSettings {
    pub fn get_builder(&self) -> WindowBuilder {
        WindowBuilder::new()
            .with_title(self.title.as_ref())
            .with_inner_size({
                let inner_size = self.inner_size.map(|x| std::cmp::max(1, x));
                PhysicalSize::<u32>::from(inner_size.into_array())
            })
            .with_fullscreen(self.fullscreen.clone())
            .with_always_on_top(self.always_on_top)
            .with_decorations(self.decorations)
            .with_maximized(self.maximized)
            .with_resizable(self.resizable)
            .with_visible(self.visible)
    }

    pub fn apply_difference(&mut self, new: &WindowSettings, window: &Window) {
        if self.title != new.title {
            window.set_title(&new.title);
        }

        if self.inner_size != new.inner_size {
            let inner_size = new.inner_size.map(|x| std::cmp::max(1, x));
            window.set_inner_size(PhysicalSize::<u32>::from(inner_size.into_array()));
        }

        if self.fullscreen != new.fullscreen {
            window.set_fullscreen(new.fullscreen.clone());
        }

        if self.always_on_top != new.always_on_top {
            window.set_always_on_top(new.always_on_top);
        }

        if self.cursor_grab != new.cursor_grab {
            let _result = window.set_cursor_grab(new.cursor_grab);
        }

        if self.cursor_visible != new.cursor_visible {
            window.set_cursor_visible(new.cursor_visible);
        }

        if self.decorations != new.decorations {
            window.set_decorations(new.decorations);
        }

        if self.maximized != new.maximized {
            window.set_maximized(new.maximized);
        }

        if self.minimized != new.minimized {
            window.set_minimized(new.minimized);
        }

        if self.resizable != new.resizable {
            window.set_resizable(new.resizable);
        }

        if self.visible != new.visible {
            window.set_visible(new.visible);
        }

        *self = new.clone();
    }
}

pub struct UiState {
    title_state: text_input::State,
    width_state: text_input::State,
    width_string: String,
    height_state: text_input::State,
    height_string: String,
}

pub struct WindowNodeBehaviour {
    settings: WindowSettings,
    ui_state: UiState,
}

impl Default for WindowNodeBehaviour {
    fn default() -> Self {
        let settings = WindowSettings::default();

        Self {
            ui_state: UiState {
                title_state: Default::default(),
                width_state: Default::default(),
                width_string: settings.inner_size[0].to_string(),
                height_state: Default::default(),
                height_string: settings.inner_size[1].to_string(),
            },
            settings,
        }
    }
}

impl WindowNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            channels_input: vec![/*Channel::new("framebuffer", TextureChannelType {})*/],
            channels_output: vec![/*Channel::new("framebuffer", TextureChannelType {})*/],
        })
    }
}

impl NodeBehaviour for WindowNodeBehaviour {
    type Message = WindowMessage;
    type State = State;

    fn name(&self) -> &str {
        "Window"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                match message {
                    WindowMessage::ModifyWindowSettings(modify) => (modify)(self),
                }

                vec![]
            }
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Self::Message>> {
        Some(
            Column::new()
                .theme(theme)
                .push(
                    TextInput::new(
                        &mut self.ui_state.title_state,
                        "Window Title",
                        self.settings.title.as_ref(),
                        |new_value| {
                            WindowMessage::ModifyWindowSettings(Arc::new(
                                move |node: &mut WindowNodeBehaviour| {
                                    node.settings.title = Cow::Owned(new_value.clone());
                                },
                            ))
                        },
                    )
                    .theme(theme),
                )
                .push(
                    Row::new()
                        .theme(theme)
                        .push(
                            TextInput::new(
                                &mut self.ui_state.width_state,
                                "Width",
                                self.ui_state.width_string.as_ref(),
                                |new_value| {
                                    WindowMessage::ModifyWindowSettings(Arc::new(
                                        move |node: &mut WindowNodeBehaviour| {
                                            if let Ok(new_value) = new_value.parse::<u32>() {
                                                node.settings.inner_size[0] = new_value;
                                            }

                                            node.ui_state.width_string = new_value.clone();
                                        },
                                    ))
                                },
                            )
                            .theme(theme),
                        )
                        .push(
                            TextInput::new(
                                &mut self.ui_state.height_state,
                                "Height",
                                self.ui_state.height_string.as_ref(),
                                |new_value| {
                                    WindowMessage::ModifyWindowSettings(Arc::new(
                                        move |node: &mut WindowNodeBehaviour| {
                                            if let Ok(new_value) = new_value.parse::<u32>() {
                                                node.settings.inner_size[1] = new_value;
                                            }

                                            node.ui_state.height_string = new_value.clone();
                                        },
                                    ))
                                },
                            )
                            .theme(theme),
                        ),
                )
                .push(
                    Checkbox::new(self.settings.always_on_top, "Always on top", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.always_on_top = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.cursor_grab, "Grab cursor", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.cursor_grab = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.cursor_visible, "Cursor visible", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.cursor_visible = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.decorations, "Decorations", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.decorations = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.maximized, "Maximized", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.maximized = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.minimized, "Minimized", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.minimized = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.resizable, "Resizable", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.resizable = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .push(
                    Checkbox::new(self.settings.visible, "Visible", |new_value| {
                        WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.visible = new_value,
                        ))
                    })
                    .theme(theme),
                )
                .into(),
        )
    }

    fn create_state_initializer(&self) -> Option<Self::FnStateInitializer> {
        Some(Box::new(|_context: &ApplicationContext| State::default()))
    }

    fn create_executor(&self) -> Self::FnExecutor {
        let settings = self.settings.clone();
        Box::new(move |mut context: ExecutionContext<'_, State>| {
            let state = context.state.take().unwrap();

            if state.window.is_none() {
                if let Some(window_receiver) = state.window_receiver.as_mut() {
                    // The window creation task has been sent, poll the response.
                    if let Ok(window) = window_receiver.try_recv() {
                        state.window = Some(WindowSurface::from(window, &context));
                    }
                } else {
                    // If the window creation task was not sent yet, send it.
                    let window_attributes = settings.get_builder().window;
                    let (window_sender, window_receiver) = flume::unbounded();
                    let task = Box::new(move |window_target: &EventLoopWindowTarget<crate::Message>| {
                        let mut builder = WindowBuilder::new();
                        builder.window = window_attributes;
                        let window = builder.build(window_target).unwrap();
                        let _result = window_sender.send(window);
                    });
                    let _result = context.application_context.main_thread_task_sender.send(task);
                    state.window_receiver = Some(window_receiver);
                }
            }

            if let Some(window) = state.window.as_mut() {
                let recreate_swapchain = state.current_settings.inner_size != settings.inner_size;

                state.current_settings.apply_difference(&settings, &window.window);

                if window.swapchain.is_none() || recreate_swapchain {
                    window.swapchain = Some(context.application_context.renderer.device.create_swap_chain(
                        &window.surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                            format: wgpu::TextureFormat::Bgra8UnormSrgb,
                            width: state.current_settings.inner_size[0],
                            height: state.current_settings.inner_size[1],
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                    ));
                }

                // Drop the previous swapchain frame, presenting it.
                window.swapchain_frame = None;
                let swapchain = window.swapchain.as_mut().unwrap();
                // Unwrap safe, because we made sure to drop the previous frame.
                let frame = swapchain.get_current_frame().unwrap();
                window.swapchain_frame = Some(frame);
            }
        })
    }
}

#[derive(Debug)]
pub struct WindowSurface {
    window: Window,
    surface: wgpu::Surface,
    swapchain: Option<wgpu::SwapChain>,
    swapchain_frame: Option<wgpu::SwapChainFrame>,
}

impl WindowSurface {
    pub fn from(window: Window, context: &ExecutionContext<'_, State>) -> Self {
        Self {
            surface: unsafe { context.application_context.renderer.instance.create_surface(&window) },
            window,
            swapchain: None,
            swapchain_frame: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct State {
    current_settings: WindowSettings,
    window_receiver: Option<Receiver<Window>>,
    window: Option<WindowSurface>,
}
