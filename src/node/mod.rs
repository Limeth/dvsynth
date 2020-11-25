use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

pub use ty::*;

pub mod ty;

pub mod behaviour;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl<'a> From<&'a Channel> for ChannelRef<'a> {
    fn from(other: &'a Channel) -> Self {
        Self {
            title: &other.title,
            description: other.description.as_ref().map(String::as_str),
            ty: &other.ty,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeConfiguration {
    pub channels_input: Vec<Channel>,
    pub channels_output: Vec<Channel>,
}

impl NodeConfiguration {
    pub fn channels(&self, direction: ChannelDirection) -> &Vec<Channel> {
        match direction {
            ChannelDirection::In => &self.channels_input,
            ChannelDirection::Out => &self.channels_output,
        }
    }

    pub fn channels_mut(&mut self, direction: ChannelDirection) -> &mut Vec<Channel> {
        match direction {
            ChannelDirection::In => &mut self.channels_input,
            ChannelDirection::Out => &mut self.channels_output,
        }
    }

    pub fn channel(&self, direction: ChannelDirection, index: usize) -> &Channel {
        &self.channels(direction)[index]
    }

    pub fn channel_mut(&mut self, direction: ChannelDirection, index: usize) -> &mut Channel {
        &mut self.channels_mut(direction)[index]
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
    pub fn zeroed(ty: &TypeEnum) -> Self {
        Self { data: vec![0_u8; ty.value_size()].into_boxed_slice() }
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
                .map(|channel| ChannelValue::zeroed(&channel.ty))
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
