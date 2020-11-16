use super::*;
use crate::style;
use iced::pick_list::{PickList, State as PickListState};
use iced::text_input::{State as TextInputState, TextInput};
use iced::{Align, Length, Row};

#[derive(Default)]
pub struct CounterNodeBehaviour;

impl NodeBehaviour for CounterNodeBehaviour {
    type Message = ();
    type State = State;

    fn name(&self) -> &str {
        "Counter"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![NodeCommand::Configure(NodeConfiguration {
                channels_input: vec![],
                channels_output: vec![Channel::new("count", PrimitiveChannelType::U32)],
            })],
            NodeEvent::Message(_) => vec![],
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Self::Message>> {
        None
    }

    fn create_state_initializer(&self) -> Option<Self::FnStateInitializer> {
        Some(Box::new(|_context: &ExecutionContext| State::default()))
    }

    fn create_executor(&self) -> Self::FnExecutor {
        Box::new(
            |_context: &ExecutionContext,
             state: Option<&mut Self::State>,
             _inputs: &ChannelValueRefs,
             outputs: &mut ChannelValues| {
                let state = state.unwrap();
                let mut cursor = Cursor::new(outputs[0].as_mut());

                cursor.write_u32::<LittleEndian>(state.count).unwrap();

                state.count += 1;
            },
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct State {
    count: u32,
}
