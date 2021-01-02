use crate::node::PrimitiveChannelValue;
use crate::{
    node::{
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeStateClosure,
        },
        BytesRefExt, Channel, NodeConfiguration, OptionRefMutExt, PrimitiveType, PrimitiveTypeEnum,
    },
    style::{Theme, Themeable},
};
use byteorder::LittleEndian;
use iced::{
    pick_list::{self, PickList},
    Element,
};
use iced::{Align, Container, Length, Row};
use std::io::Cursor;
use std::ops::{Add, Div, Mul, Sub};

#[derive(Debug, Clone)]
pub enum BinaryOpMessage {
    UpdateType(PrimitiveTypeEnum),
    UpdateOp(BinaryOp),
}

#[derive(Clone, Debug)]
pub struct BinaryOpNodeBehaviour {
    pub pick_list_ty_state: pick_list::State<PrimitiveTypeEnum>,
    pub pick_list_ty_value: PrimitiveTypeEnum,
    pub pick_list_op_state: pick_list::State<BinaryOp>,
    pub op: BinaryOp,
}

impl Default for BinaryOpNodeBehaviour {
    fn default() -> Self {
        Self {
            op: BinaryOp::Add,
            pick_list_ty_state: Default::default(),
            pick_list_ty_value: PrimitiveTypeEnum::F32,
            pick_list_op_state: Default::default(),
        }
    }
}

impl BinaryOpNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(
            NodeConfiguration::default()
                .with_input_value(Channel::new("lhs", self.pick_list_ty_value))
                .with_input_value(Channel::new("rhs", self.pick_list_ty_value))
                .with_output_value(Channel::new("result", self.pick_list_ty_value)),
        )
    }
}

impl NodeBehaviour for BinaryOpNodeBehaviour {
    type Message = BinaryOpMessage;

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
                            &PrimitiveTypeEnum::VALUES[..],
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

    fn create_state<'state>(&self, application_context: &ApplicationContext) -> Self::State<'state> {
        NodeStateClosure::new(
            self,
            application_context,
            (),
            move |behaviour: &Self, _application_context: &ApplicationContext, _persistent: &mut ()| {
                // Executed when the node settings have been changed to create the following
                // executor closure.
                let pick_list_ty_value = behaviour.pick_list_ty_value;
                let op = behaviour.op;

                Box::new(move |context: ExecutionContext<'_, 'state>, _persistent: &mut ()| {
                    // Executed once per graph execution.
                    let lhs = pick_list_ty_value
                        .read::<LittleEndian, _>(&context.inputs[0].as_bytes().unwrap())
                        .unwrap();
                    let rhs = pick_list_ty_value
                        .read::<LittleEndian, _>(&context.inputs[1].as_bytes().unwrap())
                        .unwrap();
                    let result = op.apply_dyn(lhs, rhs);
                    context.outputs[0]
                        .replace_with_bytes(context.allocator_handle, |bytes| {
                            let mut output_cursor = Cursor::new(bytes);

                            // dbg!(result);

                            result.write::<LittleEndian>(&mut output_cursor).unwrap();
                        })
                        .unwrap();
                }) as Box<dyn ExecutorClosure<'state> + 'state>
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
