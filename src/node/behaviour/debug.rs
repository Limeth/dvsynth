use crate::{
    Element,
    node::{
        BytesRefExt, Channel, NodeConfiguration, PrimitiveTypeEnum,
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeStateClosure,
        },
    },
};
use byteorder::LittleEndian;
use iced::{Alignment, Length};
use iced::{
    widget::Row,
    widget::pick_list::PickList,
};
use transient::Static;

#[derive(Debug, Clone)]
pub enum DebugNodeMessage {
    UpdateType(PrimitiveTypeEnum),
}

#[derive(Debug, Clone)]
pub struct DebugNodeBehaviour {
    ty: PrimitiveTypeEnum,
}

impl Static for DebugNodeBehaviour {}

impl Default for DebugNodeBehaviour {
    fn default() -> Self {
        Self { ty: PrimitiveTypeEnum::F32 }
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

    fn view(&self /*, theme: &dyn Theme*/) -> Option<Element<'_, Self::Message>> {
        Some(
            Row::new()
                // .theme(theme)
                .push(
                    PickList::new(&PrimitiveTypeEnum::VALUES[..], Some(self.ty), |new_value| {
                        DebugNodeMessage::UpdateType(new_value)
                    })
                    // .theme(theme)
                    .width(Length::Fill),
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
