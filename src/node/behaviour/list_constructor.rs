use crate::node::prelude::*;
use crate::node::{BorrowedRef, BorrowedRefMut, ListDescriptor, ListType, OwnedRefMut, Unique};
use crate::{
    node::{
        behaviour::{
            ApplicationContext, ExecutionContext, ExecutorClosure, NodeBehaviour, NodeCommand, NodeEvent,
            NodeStateClosure,
        },
        Channel, NodeConfiguration, PrimitiveType,
    },
    style::{Theme, Themeable},
};
use iced::{
    button::{self, Button},
    pick_list::{self, PickList},
    Element, Text,
};
use iced::{Align, Length, Row};
use std::io::{Cursor, Write};
use std::num::NonZeroUsize;

#[derive(Debug, Clone)]
pub enum ListConstructorNodeMessage {
    UpdateType(PrimitiveType),
    AddChannel,
    RemoveChannel,
}

#[derive(Debug, Clone)]
pub struct ListConstructorNodeBehaviour {
    ty: PrimitiveType,
    channel_count: NonZeroUsize,
    pick_list_state: pick_list::State<PrimitiveType>,
    button_add_state: button::State,
    button_remove_state: button::State,
}

impl Default for ListConstructorNodeBehaviour {
    fn default() -> Self {
        Self {
            ty: PrimitiveType::F32,
            channel_count: unsafe { NonZeroUsize::new_unchecked(1) },
            pick_list_state: Default::default(),
            button_add_state: Default::default(),
            button_remove_state: Default::default(),
        }
    }
}

impl ListConstructorNodeBehaviour {
    pub fn get_configure_command(&self) -> NodeCommand {
        NodeCommand::Configure(NodeConfiguration {
            channels_input: (0..self.channel_count.get())
                .into_iter()
                .map(|channel_index| Channel::new(format!("item #{}", channel_index), self.ty))
                .collect(),
            channels_output: vec![Channel::new("list", Unique::new(ListType::new(self.ty)))],
        })
    }
}

impl NodeBehaviour for ListConstructorNodeBehaviour {
    type Message = ListConstructorNodeMessage;

    fn name(&self) -> &str {
        "ListConstructor"
    }

    fn update(&mut self, event: NodeEvent<Self::Message>) -> Vec<NodeCommand> {
        match event {
            NodeEvent::Update => vec![self.get_configure_command()],
            NodeEvent::Message(message) => {
                use ListConstructorNodeMessage::*;
                let mut commands = Vec::new();

                match message {
                    UpdateType(ty) => {
                        self.ty = ty;
                        commands.push(self.get_configure_command());
                    }
                    AddChannel => {
                        self.channel_count = NonZeroUsize::new(self.channel_count.get() + 1).unwrap();
                        commands.push(self.get_configure_command());
                    }
                    RemoveChannel => {
                        if let Some(new_value) = NonZeroUsize::new(self.channel_count.get() - 1) {
                            self.channel_count = new_value;
                            commands.push(self.get_configure_command());
                        }
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
                        &PrimitiveType::VALUES[..],
                        Some(self.ty),
                        |new_value| ListConstructorNodeMessage::UpdateType(new_value),
                    )
                    .theme(theme)
                    .width(Length::Units(64)),
                )
                .push(
                    Button::new(&mut self.button_add_state, Text::new("+"))
                        .width(Length::Fill)
                        .on_press(ListConstructorNodeMessage::AddChannel),
                )
                .push(
                    Button::new(&mut self.button_remove_state, Text::new("-"))
                        .width(Length::Fill)
                        .on_press(ListConstructorNodeMessage::RemoveChannel),
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
                // executor closure:
                let ty = behaviour.ty;

                Box::new(move |context: ExecutionContext<'_, 'state>, _persistent: &mut ()| {
                    // Executed once per graph execution.
                    {
                        let mut list: OwnedRefMut<Unique<ListType>> = context
                            .allocator_handle
                            .allocate_object::<ListType>(ListDescriptor::new_if_sized(ty).unwrap());
                        let mut list: BorrowedRefMut<ListType> = list.deref_mut();
                        list.push_item_bytes_with(|bytes| {
                            bytes.iter_mut().enumerate().for_each(|(i, byte)| *byte = i as u8);
                        })
                        .unwrap();
                        list.push_item_bytes_with(|bytes| {
                            bytes.iter_mut().enumerate().for_each(|(i, byte)| *byte = 2 * i as u8);
                        })
                        .unwrap();
                        dbg!(list.get(0).unwrap().bytes_if_sized());
                        dbg!(list.get(1).unwrap().bytes_if_sized());
                        dbg!(list.len());
                    }
                    {
                        let mut list: OwnedRefMut<Unique<ListType>> =
                            context.allocator_handle.allocate_object::<ListType>(ListDescriptor::new(
                                Unique::new(ListType::new(PrimitiveType::U8)),
                            ));
                        let mut list: BorrowedRefMut<ListType> = list.deref_mut();
                        let mut inner_list_1: OwnedRefMut<Unique<ListType>> =
                            context
                                .allocator_handle
                                .allocate_object::<ListType>(ListDescriptor::new(PrimitiveType::U8));
                        list.push(inner_list_1).unwrap();
                        let mut inner_list_2: OwnedRefMut<Unique<ListType>> =
                            context
                                .allocator_handle
                                .allocate_object::<ListType>(ListDescriptor::new(PrimitiveType::U8));
                        list.push(inner_list_2).unwrap();
                        dbg!(list.get(0).unwrap().bytes_if_sized());
                        dbg!(list.get(1).unwrap().bytes_if_sized());
                        dbg!(list.len());
                    }
                    let mut cursor = Cursor::new(context.outputs[0].as_mut());

                    for input in context.inputs.values.iter() {
                        cursor.write(input).unwrap();
                    }
                }) as Box<dyn ExecutorClosure<'state>>
            },
        )
    }
}
