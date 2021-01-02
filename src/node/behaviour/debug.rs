use crate::{
    node::{
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeStateClosure,
        },
        BytesRefExt, Channel, NodeConfiguration, PrimitiveType, PrimitiveTypeEnum,
    },
    style::{Theme, Themeable},
};
use byteorder::LittleEndian;
use iced::{
    pick_list::{self, PickList},
    Element,
};
use iced::{Align, Length, Row};

#[derive(Debug, Clone)]
pub enum DebugNodeMessage {
    UpdateType(PrimitiveTypeEnum),
}

#[derive(Debug, Clone)]
pub struct DebugNodeBehaviour {
    ty: PrimitiveTypeEnum,
    pick_list_state: pick_list::State<PrimitiveTypeEnum>,
}

impl Default for DebugNodeBehaviour {
    fn default() -> Self {
        Self { ty: PrimitiveTypeEnum::F32, pick_list_state: Default::default() }
    }
}

impl DebugNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration::default().with_borrow(Channel::new("value", self.ty)))
    }
}

impl NodeBehaviour for DebugNodeBehaviour {
    type Message = DebugNodeMessage;

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
                        &PrimitiveTypeEnum::VALUES[..],
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

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeStateClosure::new(
            self,
            application_context,
            (),
            move |behaviour: &Self, _application_context: &ApplicationContext, _persistent: &mut ()| {
                // Executed when the node settings have been changed to create the following
                // executor closure.
                let ty = behaviour.ty;

                Box::new(move |context: ExecutionContext<'_, 'state>, _persistent: &mut ()| {
                    // Executed once per graph execution.
                    let value = ty.read::<LittleEndian, _>(&context.borrows[0].as_bytes().unwrap()).unwrap();
                    println!("Debug node: {:?}", value);
                }) as Box<dyn ExecutorClosure<'state> + 'state>
            },
        )
    }
}
