use super::*;
use crate::style::{self, Themeable};
use iced::pick_list::{PickList, State as PickListState};
use iced::text_input::{State as TextInputState, TextInput};
use iced::{Align, Length, Row};

#[derive(Debug, Clone)]
pub enum DebugNodeMessage {
    UpdateType(PrimitiveChannelType),
}

impl_node_behaviour_message!(DebugNodeMessage);

pub struct DebugNodeBehaviour {
    ty: PrimitiveChannelType,
    pick_list_state: PickListState<PrimitiveChannelType>,
}

impl Default for DebugNodeBehaviour {
    fn default() -> Self {
        Self { ty: PrimitiveChannelType::F32, pick_list_state: Default::default() }
    }
}

impl DebugNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            channels_input: vec![Channel::new("value", self.ty)],
            channels_output: vec![],
        })
    }
}

impl NodeBehaviour for DebugNodeBehaviour {
    type Message = DebugNodeMessage;
    type State = ();

    fn name(&self) -> &str {
        "Debug"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                let mut commands = Vec::new();

                match message {
                    DebugNodeMessage::UpdateType(ty) => {
                        self.ty = ty;
                        commands.push(self.get_configure_command());
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
                        |new_value| DebugNodeMessage::UpdateType(new_value),
                    )
                    .theme(theme)
                    .width(Length::Fill),
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
            move |_context: &ExecutionContext,
                  _state: Option<&mut Self::State>,
                  inputs: &ChannelValueRefs,
                  _outputs: &mut ChannelValues| {
                let value = ty.read::<LittleEndian, _>(&inputs[0].as_ref()).unwrap();
                println!("{:?}", value);
            },
        )
    }
}
