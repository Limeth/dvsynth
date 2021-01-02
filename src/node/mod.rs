use crate::graph::{ChannelIdentifier, Connection, EdgeEndpoint, NodeIndex};
use crate::util::StrokeType;
use std::cmp::Ordering;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

pub use ty::*;

pub mod ty;

pub mod behaviour;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelDirection {
    In,
    Out,
}

impl ChannelDirection {
    pub fn inverse(self) -> Self {
        match self {
            ChannelDirection::In => ChannelDirection::Out,
            ChannelDirection::Out => ChannelDirection::In,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ConnectionPassBy {
    Immutable,
    Mutable,
}

impl ConnectionPassBy {
    pub fn can_be_downgraded_to(self, to: Self) -> bool {
        self >= to
    }

    pub fn get_stroke_type(self) -> StrokeType {
        match self {
            ConnectionPassBy::Immutable => StrokeType::Dotted { gap_length: 5.0 },
            ConnectionPassBy::Mutable => StrokeType::Contiguous,
        }
    }

    pub fn derive_connection_pass_by(
        is_aliased: &dyn Fn(ChannelIdentifier) -> bool,
        connection: &Connection,
    ) -> ConnectionPassBy {
        std::cmp::min(
            Self::derive_output_connection_pass_by(is_aliased, connection.from()),
            Self::derive_input_connection_pass_by(connection.to()),
        )
    }

    pub fn derive_pending_connection_pass_by(
        is_aliased: &dyn Fn(ChannelIdentifier) -> bool,
        channel: ChannelIdentifier,
    ) -> ConnectionPassBy {
        match channel.channel_direction {
            ChannelDirection::Out => Self::derive_output_connection_pass_by(is_aliased, channel),
            ChannelDirection::In => Self::derive_input_connection_pass_by(channel),
        }
    }

    pub fn derive_input_connection_pass_by(to: ChannelIdentifier) -> ConnectionPassBy {
        ConnectionPassBy::from(to.pass_by)
    }

    pub fn derive_output_connection_pass_by(
        is_aliased: &dyn Fn(ChannelIdentifier) -> bool,
        from: ChannelIdentifier,
    ) -> ConnectionPassBy {
        let aliased = (is_aliased)(from);

        std::cmp::min(
            ConnectionPassBy::from(from.pass_by),
            if aliased { ConnectionPassBy::Immutable } else { ConnectionPassBy::Mutable },
        )
    }
}

impl From<ChannelPassBy> for ConnectionPassBy {
    fn from(channel: ChannelPassBy) -> Self {
        match channel {
            ChannelPassBy::SharedReference => ConnectionPassBy::Immutable,
            ChannelPassBy::MutableReference | ChannelPassBy::Value => ConnectionPassBy::Mutable,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ChannelPassBy {
    SharedReference,
    MutableReference,
    Value,
}

impl ChannelPassBy {
    pub fn get_category_index(self) -> usize {
        self as u8 as usize
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Channel {
    pub title: String,
    pub description: Option<String>,
    pub ty: TypeEnum,
}

impl Channel {
    pub fn new(title: impl ToString, ty: impl Into<TypeEnum>) -> Self {
        Self { title: title.to_string(), description: None, ty: ty.into() }
    }

    pub fn with_description(mut self, description: impl ToString) -> Self {
        self.description = Some(description.to_string());
        self
    }
}

pub struct ChannelRef<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub ty: &'a TypeEnum,
    pub edge_endpoint: EdgeEndpoint,
    pub direction: ChannelDirection,
}

impl<'a> ChannelRef<'a> {
    fn from(other: &'a Channel, edge_endpoint: EdgeEndpoint, direction: ChannelDirection) -> Self {
        Self {
            title: &other.title,
            description: other.description.as_ref().map(String::as_str),
            ty: &other.ty,
            edge_endpoint,
            direction,
        }
    }

    pub fn into_identifier(&self, node_index: NodeIndex) -> ChannelIdentifier {
        self.edge_endpoint.into_undirected_identifier(node_index).into_directed(self.direction)
    }
}

pub struct ChannelRefMut<'a> {
    pub title: &'a mut str,
    pub description: Option<&'a mut String>,
    pub ty: &'a mut TypeEnum,
    pub edge_endpoint: EdgeEndpoint,
    pub direction: ChannelDirection,
}

impl<'a> ChannelRefMut<'a> {
    fn from(other: &'a mut Channel, edge_endpoint: EdgeEndpoint, direction: ChannelDirection) -> Self {
        Self {
            title: &mut other.title,
            description: other.description.as_mut(),
            ty: &mut other.ty,
            edge_endpoint,
            direction,
        }
    }

    pub fn into_identifier(&self, node_index: NodeIndex) -> ChannelIdentifier {
        self.edge_endpoint.into_undirected_identifier(node_index).into_directed(self.direction)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeConfiguration {
    pub channels_by_shared_reference: Vec<Channel>,
    pub channels_by_mutable_reference: Vec<Channel>,
    pub input_channels_by_value: Vec<Channel>,
    pub output_channels_by_value: Vec<Channel>,
}

impl NodeConfiguration {
    pub fn with_borrow(mut self, channel: Channel) -> Self {
        self.channels_by_shared_reference.push(channel);
        self
    }

    pub fn with_borrow_mut(mut self, channel: Channel) -> Self {
        self.channels_by_mutable_reference.push(channel);
        self
    }

    pub fn with_input_value(mut self, channel: Channel) -> Self {
        self.input_channels_by_value.push(channel);
        self
    }

    pub fn with_output_value(mut self, channel: Channel) -> Self {
        self.output_channels_by_value.push(channel);
        self
    }

    pub fn get_global_channel_index(&self, endpoint: EdgeEndpoint) -> usize {
        let mut index = endpoint.channel_index;

        if endpoint.pass_by > ChannelPassBy::SharedReference {
            index += self.channels_by_shared_reference.len();

            if endpoint.pass_by > ChannelPassBy::MutableReference {
                index += self.channels_by_mutable_reference.len();
            }
        }

        index
    }

    pub fn channels(&self, direction: ChannelDirection) -> impl Iterator<Item = ChannelRef<'_>> {
        self.channels_by_shared_reference
            .iter()
            .enumerate()
            .map(move |(channel_index, channel)| {
                ChannelRef::from(
                    channel,
                    EdgeEndpoint { channel_index, pass_by: ChannelPassBy::SharedReference },
                    direction,
                )
            })
            .chain(self.channels_by_mutable_reference.iter().enumerate().map(
                move |(channel_index, channel)| {
                    ChannelRef::from(
                        channel,
                        EdgeEndpoint { channel_index, pass_by: ChannelPassBy::MutableReference },
                        direction,
                    )
                },
            ))
            .chain(
                match direction {
                    ChannelDirection::In => self.input_channels_by_value.iter(),
                    ChannelDirection::Out => self.output_channels_by_value.iter(),
                }
                .enumerate()
                .map(move |(channel_index, channel)| {
                    ChannelRef::from(
                        channel,
                        EdgeEndpoint { channel_index, pass_by: ChannelPassBy::Value },
                        direction,
                    )
                }),
            )
    }

    pub fn channels_mut(&mut self, direction: ChannelDirection) -> impl Iterator<Item = ChannelRefMut<'_>> {
        self.channels_by_shared_reference
            .iter_mut()
            .enumerate()
            .map(move |(channel_index, channel)| {
                ChannelRefMut::from(
                    channel,
                    EdgeEndpoint { channel_index, pass_by: ChannelPassBy::SharedReference },
                    direction,
                )
            })
            .chain(self.channels_by_mutable_reference.iter_mut().enumerate().map(
                move |(channel_index, channel)| {
                    ChannelRefMut::from(
                        channel,
                        EdgeEndpoint { channel_index, pass_by: ChannelPassBy::MutableReference },
                        direction,
                    )
                },
            ))
            .chain(
                match direction {
                    ChannelDirection::In => self.input_channels_by_value.iter_mut(),
                    ChannelDirection::Out => self.output_channels_by_value.iter_mut(),
                }
                .enumerate()
                .map(move |(channel_index, channel)| {
                    ChannelRefMut::from(
                        channel,
                        EdgeEndpoint { channel_index, pass_by: ChannelPassBy::Value },
                        direction,
                    )
                }),
            )
    }

    pub fn channel(&self, direction: ChannelDirection, edge_endpoint: EdgeEndpoint) -> ChannelRef<'_> {
        match edge_endpoint.pass_by {
            ChannelPassBy::SharedReference => ChannelRef::from(
                &self.channels_by_shared_reference[edge_endpoint.channel_index],
                edge_endpoint,
                direction,
            ),
            ChannelPassBy::MutableReference => ChannelRef::from(
                &self.channels_by_mutable_reference[edge_endpoint.channel_index],
                edge_endpoint,
                direction,
            ),
            ChannelPassBy::Value => match direction {
                ChannelDirection::In => ChannelRef::from(
                    &self.input_channels_by_value[edge_endpoint.channel_index],
                    edge_endpoint,
                    direction,
                ),
                ChannelDirection::Out => ChannelRef::from(
                    &self.output_channels_by_value[edge_endpoint.channel_index],
                    edge_endpoint,
                    direction,
                ),
            },
        }
    }

    pub fn channel_mut(
        &mut self,
        direction: ChannelDirection,
        edge_endpoint: EdgeEndpoint,
    ) -> ChannelRefMut<'_> {
        match edge_endpoint.pass_by {
            ChannelPassBy::SharedReference => ChannelRefMut::from(
                &mut self.channels_by_shared_reference[edge_endpoint.channel_index],
                edge_endpoint,
                direction,
            ),
            ChannelPassBy::MutableReference => ChannelRefMut::from(
                &mut self.channels_by_mutable_reference[edge_endpoint.channel_index],
                edge_endpoint,
                direction,
            ),
            ChannelPassBy::Value => match direction {
                ChannelDirection::In => ChannelRefMut::from(
                    &mut self.input_channels_by_value[edge_endpoint.channel_index],
                    edge_endpoint,
                    direction,
                ),
                ChannelDirection::Out => ChannelRefMut::from(
                    &mut self.output_channels_by_value[edge_endpoint.channel_index],
                    edge_endpoint,
                    direction,
                ),
            },
        }
    }
}

/// Data passed from/to a channel
#[derive(Clone)]
pub struct ChannelValue {
    pub data: Box<[u8]>,
}

impl ChannelValue {
    pub fn as_channel_value_ref(&self) -> ChannelValueRef {
        ChannelValueRef { data: &self.data }
    }

    pub fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

impl ChannelValue {
    pub fn zeroed(ty: &TypeEnum) -> Option<Self> {
        ty.value_size_if_sized().map(|value_size| Self { data: vec![0_u8; value_size].into_boxed_slice() })
    }
}

impl Deref for ChannelValue {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

impl DerefMut for ChannelValue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut()
    }
}

#[derive(Clone, Copy)]
pub struct ChannelValueRef<'a> {
    pub data: &'a [u8],
}

impl<'a> Deref for ChannelValueRef<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

/// `ChannelValue`s for multiple channels
pub struct ChannelValues {
    pub values: Box<[ChannelValue]>,
}

impl ChannelValues {
    pub fn zeroed(channels: &[Channel]) -> Self {
        Self {
            values: channels
                .iter()
                .map(|channel| {
                    ChannelValue::zeroed(&channel.ty).unwrap_or_else(|| {
                        dbg!(&channel);
                        panic!()
                    })
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        }
    }
}

impl Index<usize> for ChannelValues {
    type Output = ChannelValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl IndexMut<usize> for ChannelValues {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}

pub struct ChannelValueRefs<'a> {
    pub values: Box<[ChannelValueRef<'a>]>,
}

impl<'a> Index<usize> for ChannelValueRefs<'a> {
    type Output = ChannelValueRef<'a>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.values[index]
    }
}

impl<'a> IndexMut<usize> for ChannelValueRefs<'a> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.values[index]
    }
}
