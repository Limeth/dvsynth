use super::*;
use crate::style;
use flume::{self, Receiver, Sender};
use iced::widget::checkbox::Checkbox;
use iced::widget::text_input::{self, TextInput};
use iced::{Align, Column, Container, Length, Row};
use iced_winit::winit;
use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;
use vek::Vec2;
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
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
    always_on_top: bool,
    cursor_grab: bool,
    cursor_visible: bool,
    decorations: bool,
    fullscreen: Option<Fullscreen>,
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
            always_on_top: false,
            cursor_grab: false,
            cursor_visible: true,
            decorations: true,
            fullscreen: None,
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
            .with_always_on_top(self.always_on_top)
            .with_decorations(self.decorations)
            .with_fullscreen(self.fullscreen.clone())
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

        if self.fullscreen != new.fullscreen {
            window.set_fullscreen(new.fullscreen.clone());
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
    fn name(&self) -> &str {
        "Window"
    }

    fn update(&mut self, event: NodeEvent) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                let message = message.downcast::<WindowMessage>().unwrap();

                match *message {
                    WindowMessage::ModifyWindowSettings(modify) => (modify)(self),
                }

                vec![]
            }
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>> {
        Some(
            Column::new()
                .spacing(style::consts::SPACING_VERTICAL)
                .push(
                    TextInput::new(
                        &mut self.ui_state.title_state,
                        "Window Title",
                        self.settings.title.as_ref(),
                        |new_value| {
                            Box::new(WindowMessage::ModifyWindowSettings(Arc::new(
                                move |node: &mut WindowNodeBehaviour| {
                                    node.settings.title = Cow::Owned(new_value.clone());
                                },
                            ))) as Box<dyn NodeBehaviourMessage>
                        },
                    )
                    .size(style::consts::TEXT_SIZE_REGULAR)
                    .padding(style::consts::SPACING_VERTICAL)
                    .style(theme.text_input()),
                )
                .push(
                    Row::new()
                        .spacing(style::consts::SPACING_HORIZONTAL)
                        .push(
                            TextInput::new(
                                &mut self.ui_state.width_state,
                                "Width",
                                self.ui_state.width_string.as_ref(),
                                |new_value| {
                                    Box::new(WindowMessage::ModifyWindowSettings(Arc::new(
                                        move |node: &mut WindowNodeBehaviour| {
                                            if let Ok(new_value) = new_value.parse::<u32>() {
                                                node.settings.inner_size[0] = new_value;
                                            }

                                            node.ui_state.width_string = new_value.clone();
                                        },
                                    ))) as Box<dyn NodeBehaviourMessage>
                                },
                            )
                            .size(style::consts::TEXT_SIZE_REGULAR)
                            .padding(style::consts::SPACING_VERTICAL)
                            .style(theme.text_input()),
                        )
                        .push(
                            TextInput::new(
                                &mut self.ui_state.height_state,
                                "Height",
                                self.ui_state.height_string.as_ref(),
                                |new_value| {
                                    Box::new(WindowMessage::ModifyWindowSettings(Arc::new(
                                        move |node: &mut WindowNodeBehaviour| {
                                            if let Ok(new_value) = new_value.parse::<u32>() {
                                                node.settings.inner_size[1] = new_value;
                                            }

                                            node.ui_state.height_string = new_value.clone();
                                        },
                                    ))) as Box<dyn NodeBehaviourMessage>
                                },
                            )
                            .size(style::consts::TEXT_SIZE_REGULAR)
                            .padding(style::consts::SPACING_VERTICAL)
                            .style(theme.text_input()),
                        ),
                )
                .push(
                    Checkbox::new(self.settings.always_on_top, "Always on top", |new_value| {
                        Box::new(WindowMessage::ModifyWindowSettings(Arc::new(
                            move |node: &mut WindowNodeBehaviour| node.settings.always_on_top = new_value,
                        ))) as Box<dyn NodeBehaviourMessage>
                    })
                    .size(style::consts::TEXT_SIZE_REGULAR)
                    .text_size(style::consts::TEXT_SIZE_REGULAR)
                    .spacing(style::consts::SPACING_HORIZONTAL)
                    .style(theme.checkbox()),
                )
                .into(),
        )
    }

    fn create_state_initializer(&self) -> Option<Arc<NodeStateInitializer>> {
        Some(Arc::new(|context: &ExecutionContext| Box::new(State::default()) as Box<dyn NodeExecutorState>))
    }

    fn create_executor(&self) -> Arc<NodeExecutor> {
        let settings = self.settings.clone();
        Arc::new(
            move |context: &ExecutionContext,
                  state: Option<&mut dyn NodeExecutorState>,
                  inputs: &ChannelValueRefs,
                  outputs: &mut ChannelValues| {
                let state = state.unwrap().downcast_mut::<State>().unwrap();

                if state.window.is_none() {
                    if let Some(window_receiver) = state.window_receiver.as_mut() {
                        // The window creation task has been sent, poll the response.
                        if let Ok(window) = window_receiver.try_recv() {
                            state.window = Some(window);
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
                        let _result = context.main_thread_task_sender.send(task);
                        state.window_receiver = Some(window_receiver);

                        println!("Create requested.");
                    }
                }

                if let Some(window) = state.window.as_mut() {
                    state.current_settings.apply_difference(&settings, &window);
                }
            },
        )
    }
}

#[derive(Debug, Default)]
struct State {
    window_receiver: Option<Receiver<Window>>,
    window: Option<Window>,
    current_settings: WindowSettings,
}
