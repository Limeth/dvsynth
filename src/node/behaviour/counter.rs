use crate::{
    node::{
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeExecutorStateClosure,
        },
        Channel, NodeConfiguration, PrimitiveType,
    },
    style::Theme,
};
use byteorder::{LittleEndian, WriteBytesExt};
use iced::Element;
use std::io::Cursor;

#[derive(Clone, Debug, Default)]
pub struct CounterNodeBehaviour;

impl NodeBehaviour for CounterNodeBehaviour {
    type Message = ();
    type State<'state> = NodeExecutorStateClosure<'state, Self, Transient>;

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

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeExecutorStateClosure::new(
            self,
            application_context,
            Transient::default(),
            move |_behaviour: &Self,
                  _application_context: &ApplicationContext,
                  _transient: &mut Transient| {
                // Executed when the node settings have been changed to create the following
                // executor closure.
                Box::new(move |context: ExecutionContext<'_, 'state>, transient: &mut Transient| {
                    // Executed once per graph execution.
                    let mut cursor = Cursor::new(context.outputs[0].as_mut());

                    cursor.write_u32::<LittleEndian>(transient.count).unwrap();

                    transient.count += 1;
                }) as Box<dyn ExecutorClosure<'state, Transient> + 'state>
            },
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct Transient {
    count: u32,
}
