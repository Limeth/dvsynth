use crate::node::PrimitiveChannelValue;
use crate::{
    graph::ApplicationContext,
    node::{
        behaviour::{
            ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent, NodeStateClosure,
        },
        Channel, NodeConfiguration, OptionRefMutExt, PrimitiveType, PrimitiveTypeEnum,
    },
    style::{Theme, Themeable},
};
use byteorder::LittleEndian;
use iced::{
    pick_list::{self, PickList},
    text_input::{self, TextInput},
    Element,
};
use iced::{Align, Length, Row};
use std::io::Cursor;

#[derive(Debug, Clone)]
pub enum ConstantNodeMessage {
    UpdateType(PrimitiveTypeEnum),
    UpdateValue(String),
}

#[derive(Clone, Debug)]
pub struct ConstantNodeBehaviour {
    value: PrimitiveChannelValue,
    pick_list_state: pick_list::State<PrimitiveTypeEnum>,
    text_input_state: text_input::State,
    text_input_value: String,
    text_input_placeholder: String,
}

impl Default for ConstantNodeBehaviour {
    fn default() -> Self {
        Self {
            value: PrimitiveTypeEnum::F32.default_value(),
            pick_list_state: Default::default(),
            text_input_state: Default::default(),
            text_input_value: Default::default(),
            text_input_placeholder: PrimitiveTypeEnum::F32.default_value().value_to_string(),
        }
    }
}

impl ConstantNodeBehaviour {
    pub fn new(value: impl Into<PrimitiveChannelValue>) -> Self {
        let mut result = Self::default();
        result.set_value(value.into());
        result.text_input_value = result.value.value_to_string();
        result
    }

    pub fn set_value(&mut self, value: PrimitiveChannelValue) {
        self.value = value;
        self.text_input_placeholder = value.ty().default_value().value_to_string();
    }

    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(
            NodeConfiguration::default().with_output_value(Channel::new("value", self.value.ty())),
        )
    }
}

impl NodeBehaviour for ConstantNodeBehaviour {
    type Message = ConstantNodeMessage;

    fn name(&self) -> &str {
        "Constant"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                let mut commands = Vec::new();

                match message {
                    ConstantNodeMessage::UpdateType(ty) => {
                        let new_value =
                            ty.parse(&self.text_input_value).unwrap_or_else(|| ty.default_value());

                        self.set_value(new_value);
                        commands.push(self.get_configure_command());
                    }
                    ConstantNodeMessage::UpdateValue(new_raw_value) => {
                        self.text_input_value = new_raw_value;
                        let ty = self.value.ty();
                        let new_value =
                            ty.parse(&self.text_input_value).unwrap_or_else(|| ty.default_value());

                        self.set_value(new_value);
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
                        &PrimitiveTypeEnum::VALUES[..],
                        Some(self.value.ty()),
                        |new_value| ConstantNodeMessage::UpdateType(new_value),
                    )
                    .theme(theme)
                    .width(Length::Units(64)),
                )
                .push(
                    TextInput::new(
                        &mut self.text_input_state,
                        &self.text_input_placeholder,
                        &self.text_input_value,
                        |new_raw_value| ConstantNodeMessage::UpdateValue(new_raw_value),
                    )
                    .theme(theme)
                    .width(Length::Fill),
                )
                .align_items(Align::Center)
                .width(Length::Fill)
                .into(),
        )
    }

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeStateClosure::new(
            self,
            application_context,
            (),
            move |behaviour: &Self, _application_context: &ApplicationContext, _persistent: &mut ()| {
                // Executed when the node settings have been changed to create the following
                // executor closure.

                // Copy the constant value from the GUI settings.
                let value = behaviour.value;

                Box::new(move |context: ExecutionContext<'_, 'state>, _persistent: &mut ()| {
                    // Executed once per graph execution.
                    context.outputs[0]
                        .replace_with_bytes(context.allocator_handle, |bytes| {
                            let mut cursor = Cursor::new(bytes);

                            value.write::<LittleEndian>(&mut cursor).unwrap();
                            dbg!(value);
                        })
                        .unwrap();
                }) as Box<dyn ExecutorClosure<'state> + 'state>
            },
        )
    }
}
