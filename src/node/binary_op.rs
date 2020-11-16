use super::*;
use crate::style::{self, Themeable};
use iced::{Align, Container, Length, Row};

#[derive(Debug, Clone)]
pub enum BinaryOpMessage {
    UpdateType(PrimitiveChannelType),
    UpdateOp(BinaryOp),
}

impl_node_behaviour_message!(BinaryOpMessage);

pub struct BinaryOpNodeBehaviour {
    pub pick_list_ty_state: PickListState<PrimitiveChannelType>,
    pub pick_list_ty_value: PrimitiveChannelType,
    pub pick_list_op_state: PickListState<BinaryOp>,
    pub op: BinaryOp,
}

impl Default for BinaryOpNodeBehaviour {
    fn default() -> Self {
        Self {
            op: BinaryOp::Add,
            pick_list_ty_state: Default::default(),
            pick_list_ty_value: PrimitiveChannelType::F32,
            pick_list_op_state: Default::default(),
        }
    }
}

impl BinaryOpNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            channels_input: vec![
                Channel::new("lhs", self.pick_list_ty_value),
                Channel::new("rhs", self.pick_list_ty_value),
            ],
            channels_output: vec![Channel::new("result", self.pick_list_ty_value)],
        })
    }
}

impl NodeBehaviour for BinaryOpNodeBehaviour {
    type Message = BinaryOpMessage;
    type State = ();

    fn name(&self) -> &str {
        "Binary Operation"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                let mut commands = Vec::new();
                match message {
                    BinaryOpMessage::UpdateType(ty) => {
                        self.pick_list_ty_value = ty;
                        commands.push(self.get_configure_command());
                    }
                    BinaryOpMessage::UpdateOp(value) => {
                        self.op = value;
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
                    // Wrap PickList in a container because PickList's width resolution is buggy
                    Container::new(
                        PickList::new(
                            &mut self.pick_list_ty_state,
                            &PrimitiveChannelType::VALUES[..],
                            Some(self.pick_list_ty_value),
                            |new_value| BinaryOpMessage::UpdateType(new_value),
                        )
                        .theme(theme)
                        .width(Length::Fill),
                    )
                    .width(Length::Fill),
                )
                .push(
                    // Wrap PickList in a container because PickList's width resolution is buggy
                    Container::new(
                        PickList::new(
                            &mut self.pick_list_op_state,
                            &BinaryOp::VALUES[..],
                            Some(self.op),
                            |value| BinaryOpMessage::UpdateOp(value),
                        )
                        .theme(theme)
                        .width(Length::Fill),
                    )
                    .width(Length::Units(48)),
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
        let pick_list_ty_value = self.pick_list_ty_value;
        let op = self.op;
        Box::new(
            move |_context: &ExecutionContext,
                  _state: Option<&mut Self::State>,
                  inputs: &ChannelValueRefs,
                  outputs: &mut ChannelValues| {
                let lhs = pick_list_ty_value.read::<LittleEndian, _>(&inputs[0].as_ref()).unwrap();
                let rhs = pick_list_ty_value.read::<LittleEndian, _>(&inputs[1].as_ref()).unwrap();
                let result = op.apply_dyn(lhs, rhs);
                let mut output_cursor = Cursor::new(outputs[0].as_mut());

                // dbg!(result);

                result.write::<LittleEndian>(&mut output_cursor).unwrap();
            },
        )
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    // Or,
    // And,
    // Xor,
}

impl ToString for BinaryOp {
    fn to_string(&self) -> String {
        use BinaryOp::*;
        match self {
            Add => "+",
            Sub => "-",
            Mul => "*",
            Div => "/",
        }
        .to_string()
    }
}

impl BinaryOp {
    pub const VALUES: [BinaryOp; 4] = [BinaryOp::Add, BinaryOp::Sub, BinaryOp::Mul, BinaryOp::Div];

    pub fn apply<T>(self, lhs: T, rhs: T) -> T
    where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> {
        match self {
            BinaryOp::Add => Add::add(lhs, rhs),
            BinaryOp::Sub => Sub::sub(lhs, rhs),
            BinaryOp::Mul => Mul::mul(lhs, rhs),
            BinaryOp::Div => Div::div(lhs, rhs),
        }
    }

    pub fn apply_dyn(self, lhs: PrimitiveChannelValue, rhs: PrimitiveChannelValue) -> PrimitiveChannelValue {
        use PrimitiveChannelValue::*;
        match (lhs, rhs) {
            (U8(lhs), U8(rhs)) => U8(self.apply(lhs, rhs)),
            (U16(lhs), U16(rhs)) => U16(self.apply(lhs, rhs)),
            (U32(lhs), U32(rhs)) => U32(self.apply(lhs, rhs)),
            (U64(lhs), U64(rhs)) => U64(self.apply(lhs, rhs)),
            (U128(lhs), U128(rhs)) => U128(self.apply(lhs, rhs)),
            (I8(lhs), I8(rhs)) => I8(self.apply(lhs, rhs)),
            (I16(lhs), I16(rhs)) => I16(self.apply(lhs, rhs)),
            (I32(lhs), I32(rhs)) => I32(self.apply(lhs, rhs)),
            (I64(lhs), I64(rhs)) => I64(self.apply(lhs, rhs)),
            (I128(lhs), I128(rhs)) => I128(self.apply(lhs, rhs)),
            (F32(lhs), F32(rhs)) => F32(self.apply(lhs, rhs)),
            (F64(lhs), F64(rhs)) => F64(self.apply(lhs, rhs)),
            _ => panic!("Incompatible dynamic primitive types when trying to apply a binary operation."),
        }
    }
}
