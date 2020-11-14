use super::*;
use crate::style;
use iced::pick_list::{PickList, State as PickListState};
use iced::text_input::{State as TextInputState, TextInput};
use iced::{Align, Length, Row};

#[derive(Debug, Clone)]
pub enum ConstantNodeMessage {
    UpdateType(PrimitiveChannelType),
    UpdateValue(String),
}

impl_node_behaviour_message!(ConstantNodeMessage);

pub struct ConstantNodeBehaviour {
    value: PrimitiveChannelValue,
    pick_list_state: PickListState<PrimitiveChannelType>,
    text_input_state: TextInputState,
    text_input_value: String,
    text_input_placeholder: String,
}

impl Default for ConstantNodeBehaviour {
    fn default() -> Self {
        Self {
            value: PrimitiveChannelType::F32.default_value(),
            pick_list_state: Default::default(),
            text_input_state: Default::default(),
            text_input_value: Default::default(),
            text_input_placeholder: PrimitiveChannelType::F32.default_value().value_to_string(),
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
        NodeCommand::Configure(NodeConfiguration {
            channels_input: Vec::new(),
            channels_output: vec![Channel::new("value", self.value.ty())],
        })
    }
}

impl NodeBehaviour for ConstantNodeBehaviour {
    fn name(&self) -> &str {
        "Constant"
    }

    fn update(&mut self, event: NodeEvent) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                let mut commands = Vec::new();

                if let Ok(message) = message.downcast::<ConstantNodeMessage>() {
                    match *message {
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
                }

                commands
            }
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>> {
        Some(
            Row::new()
                .push(
                    PickList::new(
                        &mut self.pick_list_state,
                        &PrimitiveChannelType::VALUES[..],
                        Some(self.value.ty()),
                        |new_value| {
                            Box::new(ConstantNodeMessage::UpdateType(new_value))
                                as Box<dyn NodeBehaviourMessage>
                        },
                    )
                    .width(Length::Units(64))
                    .text_size(style::consts::TEXT_SIZE_REGULAR)
                    .padding(style::consts::SPACING_VERTICAL)
                    .style(theme.pick_list()),
                )
                .push(
                    TextInput::new(
                        &mut self.text_input_state,
                        &self.text_input_placeholder,
                        &self.text_input_value,
                        |new_raw_value| {
                            Box::new(ConstantNodeMessage::UpdateValue(new_raw_value))
                                as Box<dyn NodeBehaviourMessage>
                        },
                    )
                    .width(Length::Fill)
                    .size(style::consts::TEXT_SIZE_REGULAR)
                    .padding(style::consts::SPACING_VERTICAL)
                    .style(theme.text_input()),
                )
                .align_items(Align::Center)
                .width(Length::Fill)
                .spacing(style::consts::SPACING_HORIZONTAL)
                .into(),
        )
    }

    fn create_executor(&self) -> ArcNodeExecutor {
        let value = self.value;
        Arc::new(move |_inputs: &ChannelValueRefs, outputs: &mut ChannelValues| {
            let mut cursor = Cursor::new(outputs[0].as_mut());

            value.write::<LittleEndian>(&mut cursor).unwrap();
        })
    }
}
