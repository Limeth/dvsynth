use super::*;
use crate::style;
use iced::pick_list::{PickList, State as PickListState};
use iced::text_input::{State as TextInputState, TextInput};
use iced::{Align, Length, Row};

#[derive(Default)]
pub struct CounterNodeBehaviour;

impl NodeBehaviour for CounterNodeBehaviour {
    fn name(&self) -> &str {
        "Counter"
    }

    fn update(&mut self, event: NodeEvent) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![NodeCommand::Configure(NodeConfiguration {
                channels_input: vec![],
                channels_output: vec![Channel::new("count", PrimitiveChannelType::U32)],
            })],
            NodeEvent::Message(_) => vec![],
        }
    }

    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>> {
        None
    }

    fn create_state_initializer(&self) -> Option<Arc<NodeStateInitializer>> {
        Some(Arc::new(|_context: &ExecutionContext| Box::new(State::default()) as Box<dyn NodeExecutorState>))
    }

    fn create_executor(&self) -> Arc<NodeExecutor> {
        Arc::new(
            |_context: &ExecutionContext,
             state: Option<&mut dyn NodeExecutorState>,
             _inputs: &ChannelValueRefs,
             outputs: &mut ChannelValues| {
                let state = state.unwrap();
                let state = state.downcast_mut::<State>().unwrap();
                let mut cursor = Cursor::new(outputs[0].as_mut());

                cursor.write_u32::<LittleEndian>(state.count).unwrap();

                state.count += 1;
            },
        )
    }
}

#[derive(Default, Debug, Clone)]
struct State {
    count: u32,
}
