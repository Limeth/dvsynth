use iced::{
    Alignment, Length,
    widget::{Button, PickList, Row, Text},
};

use crate::{
    Element,
    node::{
        ArrayType, BytesRefExt, Channel, NodeConfiguration, OptionRefMutExt, PrimitiveTypeEnum,
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeStateClosure,
        },
    },
};
use std::io::{Cursor, Write};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub enum ArrayConstructorNodeMessage {
    UpdateType(PrimitiveTypeEnum),
    AddChannel,
    RemoveChannel,
}

#[derive(Clone, Debug)]
pub struct ArrayConstructorNodeBehaviour {
    ty: PrimitiveTypeEnum,
    channel_count: NonZeroUsize,
}

impl Default for ArrayConstructorNodeBehaviour {
    fn default() -> Self {
        Self { ty: PrimitiveTypeEnum::F32, channel_count: unsafe { NonZeroUsize::new_unchecked(1) } }
    }
}

impl ArrayConstructorNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            input_channels_by_value: (0..self.channel_count.get())
                .into_iter()
                .map(|channel_index| Channel::new(format!("item #{}", channel_index), self.ty))
                .collect(),
            output_channels_by_value: vec![Channel::new(
                "array",
                ArrayType::new_if_sized(self.ty, self.channel_count.get()).unwrap(),
            )],
            ..Default::default()
        })
    }
}

impl NodeBehaviour for ArrayConstructorNodeBehaviour {
    type Message = ArrayConstructorNodeMessage;

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

    fn view(&mut self /*, theme: &dyn Theme*/) -> Option<Element<Self::Message>> {
        Some(
            Row::new()
                .push(
                    PickList::new(&PrimitiveTypeEnum::VALUES[..], Some(self.ty), |new_value| {
                        ArrayConstructorNodeMessage::UpdateType(new_value)
                    })
                    // .theme(theme)
                    .width(Length::Fixed(64.0)),
                )
                .push(
                    Button::new(Text::new("+"))
                        .width(Length::Fill)
                        .on_press(ArrayConstructorNodeMessage::AddChannel),
                )
                .push(
                    Button::new(Text::new("-"))
                        .width(Length::Fill)
                        .on_press(ArrayConstructorNodeMessage::RemoveChannel),
                )
                .align_y(Alignment::Center)
                .width(Length::Fill)
                .into(),
        )
    }

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeStateClosure::new(
            self,
            application_context,
            (),
            move |_behaviour: &Self, _application_context: &ApplicationContext, _persistent: &mut ()| {
                // Executed when the node settings have been changed to create the following
                // executor closure.
                Box::new(move |context: ExecutionContext<'_, 'state>, _persistent: &mut ()| {
                    // Executed once per graph execution.
                    let inputs = &context.inputs;
                    context.outputs[0]
                        .replace_with_bytes(context.allocator_handle, |bytes| {
                            let mut cursor = Cursor::new(bytes);

                            for input in inputs.iter() {
                                cursor.write(input.as_bytes().unwrap()).unwrap();
                            }
                        })
                        .unwrap();
                }) as Box<dyn ExecutorClosure<'state> + 'state>
            },
        )
    }
}
