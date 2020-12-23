use crate::{
    node::{
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeExecutorStateClosure,
        },
        ArrayType, Channel, NodeConfiguration, PrimitiveType,
    },
    style::{self, Themeable},
};
use iced::pick_list::{self, PickList};
use iced::{
    button::{Button, State as ButtonState},
    Element,
};
use iced::{Align, Length, Row, Text};
use std::io::{Cursor, Write};
use std::num::NonZeroUsize;
use style::Theme;

#[derive(Debug, Clone)]
pub enum ArrayConstructorNodeMessage {
    UpdateType(PrimitiveType),
    AddChannel,
    RemoveChannel,
}

impl_node_behaviour_message!(ArrayConstructorNodeMessage);

#[derive(Clone, Debug)]
pub struct ArrayConstructorNodeBehaviour {
    ty: PrimitiveType,
    channel_count: NonZeroUsize,
    pick_list_state: pick_list::State<PrimitiveType>,
    button_add_state: ButtonState,
    button_remove_state: ButtonState,
}

impl Default for ArrayConstructorNodeBehaviour {
    fn default() -> Self {
        Self {
            ty: PrimitiveType::F32,
            channel_count: unsafe { NonZeroUsize::new_unchecked(1) },
            pick_list_state: Default::default(),
            button_add_state: Default::default(),
            button_remove_state: Default::default(),
        }
    }
}

impl ArrayConstructorNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            channels_input: (0..self.channel_count.get())
                .into_iter()
                .map(|channel_index| Channel::new(format!("item #{}", channel_index), self.ty))
                .collect(),
            channels_output: vec![Channel::new("array", ArrayType::new(self.ty, self.channel_count.get()))],
        })
    }
}

impl NodeBehaviour for ArrayConstructorNodeBehaviour {
    type Message = ArrayConstructorNodeMessage;
    type State<'state> = NodeExecutorStateClosure<'state, Self>;

    fn name(&self) -> &str {
        "ArrayConstructor"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                use ArrayConstructorNodeMessage::*;
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
                        &PrimitiveType::VALUES[..],
                        Some(self.ty),
                        |new_value| ArrayConstructorNodeMessage::UpdateType(new_value),
                    )
                    .theme(theme)
                    .width(Length::Units(64)),
                )
                .push(
                    Button::new(&mut self.button_add_state, Text::new("+"))
                        .width(Length::Fill)
                        .on_press(ArrayConstructorNodeMessage::AddChannel),
                )
                .push(
                    Button::new(&mut self.button_remove_state, Text::new("-"))
                        .width(Length::Fill)
                        .on_press(ArrayConstructorNodeMessage::RemoveChannel),
                )
                .align_items(Align::Center)
                .width(Length::Fill)
                .into(),
        )
    }

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeExecutorStateClosure::new(
            self,
            application_context,
            (),
            move |_behaviour: &Self, _application_context: &ApplicationContext, _transient: &mut ()| {
                // Executed when the node settings have been changed to create the following
                // executor closure.
                Box::new(move |context: ExecutionContext<'_, 'state>, _transient: &mut ()| {
                    // Executed once per graph execution.
                    let mut cursor = Cursor::new(context.outputs[0].as_mut());

                    for input in context.inputs.values.iter() {
                        cursor.write(input).unwrap();
                    }
                }) as Box<dyn ExecutorClosure<'state> + 'state>
            },
        )
    }
}
