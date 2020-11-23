use crate::graph::{ListAllocation, ListDescriptor, OwnedRefMut};
use crate::node::ListChannelType;
use crate::{
    node::{
        behaviour::{ExecutionContext, NodeBehaviour, NodeCommand, NodeEvent},
        Channel, NodeConfiguration, PrimitiveChannelType,
    },
    style::{Theme, Themeable},
};
use iced::{
    button::{self, Button},
    pick_list::{self, PickList},
    Element, Text,
};
use iced::{Align, Length, Row};
use std::io::{Cursor, Write};
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
    pick_list_state: pick_list::State<PrimitiveChannelType>,
    button_add_state: button::State,
    button_remove_state: button::State,
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
            channels_output: vec![Channel::new("list", ListChannelType::new(self.ty))],
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
        Box::new(move |context: ExecutionContext<'_, ()>| {
            let list: OwnedRefMut<ListAllocation> = context.allocate(ListDescriptor { item_type: ty.into() });
            let mut cursor = Cursor::new(context.outputs[0].as_mut());

            for input in context.inputs.values.iter() {
                cursor.write(input).unwrap();
            }
        })
    }
}