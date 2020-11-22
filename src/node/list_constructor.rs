use super::*;
use crate::style::{self, Themeable};
use iced::button::{Button, State as ButtonState};
use iced::pick_list::{PickList, State as PickListState};
use iced::text_input::{State as TextInputState, TextInput};
use iced::{Align, Column, Length, Row, Text};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub enum ListConstructorNodeMessage {
    UpdateType(PrimitiveChannelType),
    AddChannel,
    RemoveChannel,
}

impl_node_behaviour_message!(ListConstructorNodeMessage);

pub struct ListConstructorNodeBehaviour {
    ty: PrimitiveChannelType,
    channel_count: NonZeroUsize,
    pick_list_state: PickListState<PrimitiveChannelType>,
    button_add_state: ButtonState,
    button_remove_state: ButtonState,
}

impl Default for ListConstructorNodeBehaviour {
    fn default() -> Self {
        Self {
            ty: PrimitiveChannelType::F32,
            channel_count: unsafe { NonZeroUsize::new_unchecked(1) },
            pick_list_state: Default::default(),
            button_add_state: Default::default(),
            button_remove_state: Default::default(),
        }
    }
}

impl ListConstructorNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            channels_input: (0..self.channel_count.get())
                .into_iter()
                .map(|channel_index| Channel::new(format!("item #{}", channel_index), self.ty))
                .collect(),
            channels_output: vec![Channel::new(
                "array",
                ListChannelType::new(self.ty, self.channel_count.get()),
            )],
        })
    }
}

impl NodeBehaviour for ListConstructorNodeBehaviour {
    type Message = ListConstructorNodeMessage;
    type State = ();

    fn name(&self) -> &str {
        "ListConstructor"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                use ListConstructorNodeMessage::*;
                let mut commands = Vec::new();

                match message {
                    UpdateType(ty) => {
                        self.ty = ty;
                    }
                    AddChannel => {
                        self.channel_count = NonZeroUsize::new(self.channel_count.get() + 1).unwrap();
                        commands.push(self.get_configure_command());
                    }
                    RemoveChannel => {
                        if let Some(new_value) = NonZeroUsize::new(self.channel_count.get() - 1) {
                            self.channel_count = new_value;
                            commands.push(self.get_configure_command());
                        }
                    }
                }

                commands
            }
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Self::Message>> {
        Some(
            Row::new()
                .theme(theme)
                .push(
                    PickList::new(
                        &mut self.pick_list_state,
                        &PrimitiveChannelType::VALUES[..],
                        Some(self.ty),
                        |new_value| ListConstructorNodeMessage::UpdateType(new_value),
                    )
                    .theme(theme)
                    .width(Length::Units(64)),
                )
                .push(
                    Button::new(&mut self.button_add_state, Text::new("+"))
                        .width(Length::Fill)
                        .on_press(ListConstructorNodeMessage::AddChannel),
                )
                .push(
                    Button::new(&mut self.button_remove_state, Text::new("-"))
                        .width(Length::Fill)
                        .on_press(ListConstructorNodeMessage::RemoveChannel),
                )
                .align_items(Align::Center)
                .width(Length::Fill)
                .into(),
        )
    }

    fn create_state_initializer(&self) -> Option<Self::FnStateInitializer> {
        None
    }

    fn create_executor(&self) -> Self::FnExecutor {
        let ty = self.ty;
        Box::new(
            move |context: &ExecutionContext,
                  _state: Option<&mut Self::State>,
                  inputs: &ChannelValueRefs,
                  outputs: &mut ChannelValues| {
                let mut cursor = Cursor::new(outputs[0].as_mut());

                context.allocators.list.allocate(ListAllocation::new(ty));

                for input in inputs.values.iter() {
                    cursor.write(input);
                }
            },
        )
    }
}
