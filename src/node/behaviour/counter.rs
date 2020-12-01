use crate::graph::ApplicationContext;
use crate::{
    node::{
        behaviour::{ExecutionContext, NodeBehaviour, NodeCommand, NodeEvent},
        Channel, NodeConfiguration, PrimitiveType,
    },
    style::Theme,
};
use byteorder::{LittleEndian, WriteBytesExt};
use iced::Element;
use std::io::Cursor;

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
                channels_output: vec![Channel::new("count", PrimitiveType::U32)],
            })],
            NodeEvent::Message(_) => vec![],
        }
    }

    fn view(&mut self, _theme: &dyn Theme) -> Option<Element<Self::Message>> {
        None
    }

    fn create_state_initializer(&self) -> Option<Self::FnStateInitializer> {
        Some(Box::new(|_context: &ApplicationContext| State::default()))
    }

    fn create_executor(&self) -> Self::FnExecutor {
        Box::new(|mut context: ExecutionContext<'_, '_, State>| {
            let state = context.state.take().unwrap();
            let mut cursor = Cursor::new(context.outputs[0].as_mut());

            cursor.write_u32::<LittleEndian>(state.count).unwrap();

            state.count += 1;
        })
    }
}

#[derive(Default, Debug, Clone)]
pub struct State {
    count: u32,
}
