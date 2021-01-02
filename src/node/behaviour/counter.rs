use crate::{
    node::{
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeStateClosure,
        },
        Channel, NodeConfiguration, OptionRefMutExt, PrimitiveType, PrimitiveTypeEnum,
    },
    style::Theme,
};
use byteorder::{LittleEndian, WriteBytesExt};
use iced::Element;
use std::io::Cursor;

#[derive(Clone, Debug, Default)]
pub struct CounterNodeBehaviour;

impl NodeBehaviour for CounterNodeBehaviour {
    type State<'state> = NodeStateClosure<'state, Self, Persistent>;

    fn name(&self) -> &str {
        "Counter"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![NodeCommand::Configure(
                NodeConfiguration::default().with_output_value(Channel::new("count", PrimitiveTypeEnum::U32)),
            )],
            NodeEvent::Message(_) => vec![],
        }
    }

    fn view(&mut self, _theme: &dyn Theme) -> Option<Element<Self::Message>> {
        None
    }

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeStateClosure::new(
            self,
            application_context,
            Persistent::default(),
            move |_behaviour: &Self,
                  _application_context: &ApplicationContext,
                  _persistent: &mut Persistent| {
                // Executed when the node settings have been changed to create the following
                // executor closure.
                Box::new(move |context: ExecutionContext<'_, 'state>, persistent: &mut Persistent| {
                    // Executed once per graph execution.
                    context.outputs[0]
                        .replace_with_bytes(context.allocator_handle, |bytes| {
                            let mut cursor = Cursor::new(bytes);

                            cursor.write_u32::<LittleEndian>(persistent.count).unwrap();
                        })
                        .unwrap();

                    persistent.count += 1;
                }) as Box<dyn ExecutorClosure<'state, Persistent> + 'state>
            },
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct Persistent {
    count: u32,
}
