use super::*;
use crate::style;
use flume::{self, Receiver, Sender};
use iced::widget::text_input::{self, TextInput};
use iced::{Align, Column, Container, Length, Row};
use iced_winit::winit;
use std::borrow::Cow;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

#[derive(Debug, Clone)]
pub enum WindowMessage {
    ChangeTitle(String),
}

impl_node_behaviour_message!(WindowMessage);

#[derive(Default)]
pub struct WindowNodeBehaviour {
    title_state: text_input::State,
    title: String,
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
                    WindowMessage::ChangeTitle(new_value) => self.title = new_value,
                }

                vec![]
            }
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>> {
        Some(
            Column::new()
                .spacing(style::consts::SPACING_VERTICAL)
                .push(TextInput::new(&mut self.title_state, "Window Title", &self.title, |new_value| {
                    Box::new(WindowMessage::ChangeTitle(new_value)) as Box<dyn NodeBehaviourMessage>
                }))
                .into(),
        )
    }

    fn create_state_initializer(&self) -> Option<Arc<NodeStateInitializer>> {
        Some(Arc::new(|context: &ExecutionContext| Box::new(State::default()) as Box<dyn NodeExecutorState>))
    }

    fn create_executor(&self) -> Arc<NodeExecutor> {
        let title = if self.title.is_empty() {
            Cow::Owned(self.title.clone())
        } else {
            Cow::Borrowed("DVSynth Output Window")
        };
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
                        let window_attributes = WindowBuilder::new().window;
                        let (window_sender, window_receiver) = flume::unbounded();
                        let task = Box::new(move |window_target: &EventLoopWindowTarget<crate::Message>| {
                            let mut builder = WindowBuilder::new();
                            builder.window = window_attributes;
                            let window = builder.build(window_target).unwrap();
                            let _result = window_sender.send(window);
                        });
                        context.main_thread_task_sender.send(task);
                        state.window_receiver = Some(window_receiver);

                        println!("Create requested.");
                    }
                }

                if let Some(window) = state.window.as_mut() {
                    // if title != state.title {
                    window.set_title(&title);
                    // }
                }
            },
        )
    }
}

#[derive(Debug, Default)]
struct State {
    window_receiver: Option<Receiver<Window>>,
    window: Option<Window>,
    title: String,
}
