use crate::style::Theme;
use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use downcast_rs::{impl_downcast, Downcast};
use dyn_clone::DynClone;
use iced::widget::pick_list::{PickList, State as PickListState};
use iced::Element;
use std::any::Any;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::io::{Cursor, Read, Write};
use std::ops::{Add, Deref, DerefMut, Div, Index, IndexMut, Mul, Sub};
use std::sync::Arc;

pub use binary_op::*;
pub use constant::*;
pub use counter::*;

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

pub trait ChannelTypeTrait: Into<ChannelType> {
    fn value_size(&self) -> usize;
    fn is_abi_compatible(&self, other: &Self) -> bool;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum ChannelType {
    Primitive(PrimitiveChannelType),
    Opaque(OpaqueChannelType),
    // Tuple(Vec<Self>),
    Array(ArrayChannelType),
    // Vector { item_type: Box<Self> },
}

impl Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ChannelType::*;
        match self {
            Primitive(primitive) => f.write_fmt(format_args!("{}", primitive)),
            Opaque(opaque) => f.write_fmt(format_args!("{}", opaque)),
            Array(array) => f.write_fmt(format_args!("{}", array)),
        }
    }
}

impl ChannelTypeTrait for ChannelType {
    fn value_size(&self) -> usize {
        use ChannelType::*;
        match self {
            Primitive(primitive) => primitive.value_size(),
            Opaque(opaque) => opaque.value_size(),
            Array(array) => array.value_size(),
        }
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        use ChannelType::*;
        match (self, other) {
            (Opaque(a), Opaque(b)) => return a.is_abi_compatible(b),
            (Primitive(a), Primitive(b)) => return a.is_abi_compatible(b),
            _ => (),
        }
        if matches!(self, Array { .. }) || matches!(other, Array { .. }) {
            let a = if let Array(array) = self {
                Cow::Borrowed(array)
            } else {
                Cow::Owned(ArrayChannelType::single(self.clone()))
            };
            let b = if let Array(array) = other {
                Cow::Borrowed(array)
            } else {
                Cow::Owned(ArrayChannelType::single(other.clone()))
            };
            return a.is_abi_compatible(&b);
        }

        false
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PrimitiveKind {
    UnsignedInteger,
    SignedInteger,
    Float,
}

impl PrimitiveKind {
    pub fn is_abi_compatible(&self, other: &Self) -> bool {
        use PrimitiveKind::*;
        self == other
            || matches!((self, other), (UnsignedInteger, SignedInteger) | (SignedInteger, UnsignedInteger))
    }
}

/// Should not be used for large data storage, as the size is defined by the largest variant.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PrimitiveChannelValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
}

impl PrimitiveChannelValue {
    pub fn ty(&self) -> PrimitiveChannelType {
        use PrimitiveChannelValue::*;
        match self {
            U8(_) => PrimitiveChannelType::U8,
            U16(_) => PrimitiveChannelType::U16,
            U32(_) => PrimitiveChannelType::U32,
            U64(_) => PrimitiveChannelType::U64,
            U128(_) => PrimitiveChannelType::U128,
            I8(_) => PrimitiveChannelType::I8,
            I16(_) => PrimitiveChannelType::I16,
            I32(_) => PrimitiveChannelType::I32,
            I64(_) => PrimitiveChannelType::I64,
            I128(_) => PrimitiveChannelType::I128,
            F32(_) => PrimitiveChannelType::F32,
            F64(_) => PrimitiveChannelType::F64,
        }
    }

    pub fn value_to_string(&self) -> String {
        use PrimitiveChannelValue::*;
        match self {
            U8(value) => value.to_string(),
            U16(value) => value.to_string(),
            U32(value) => value.to_string(),
            U64(value) => value.to_string(),
            U128(value) => value.to_string(),
            I8(value) => value.to_string(),
            I16(value) => value.to_string(),
            I32(value) => value.to_string(),
            I64(value) => value.to_string(),
            I128(value) => value.to_string(),
            F32(value) => value.to_string(),
            F64(value) => value.to_string(),
        }
    }

    pub fn write<E: ByteOrder>(&self, write: &mut dyn Write) -> std::io::Result<()> {
        use PrimitiveChannelValue::*;
        match self {
            U8(value) => write.write_u8(*value),
            U16(value) => write.write_u16::<E>(*value),
            U32(value) => write.write_u32::<E>(*value),
            U64(value) => write.write_u64::<E>(*value),
            U128(value) => write.write_u128::<E>(*value),
            I8(value) => write.write_i8(*value),
            I16(value) => write.write_i16::<E>(*value),
            I32(value) => write.write_i32::<E>(*value),
            I64(value) => write.write_i64::<E>(*value),
            I128(value) => write.write_i128::<E>(*value),
            F32(value) => write.write_f32::<E>(*value),
            F64(value) => write.write_f64::<E>(*value),
        }
    }
}

macro_rules! impl_primitive_conversions {
    {
        $($enum_variant:ident ($primitive_type:ident)),*$(,)?
    } => {
        $(
            impl From<$primitive_type> for PrimitiveChannelValue {
                fn from(primitive: $primitive_type) -> Self {
                    PrimitiveChannelValue::$enum_variant(primitive)
                }
            }
        )*
    }
}

impl_primitive_conversions! {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum PrimitiveChannelType {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
}

impl PrimitiveChannelType {
    pub const VALUES: [PrimitiveChannelType; 12] = {
        use PrimitiveChannelType::*;
        [U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64]
    };

    pub fn kind(&self) -> PrimitiveKind {
        use PrimitiveChannelType::*;
        match self {
            U8 | U16 | U32 | U64 | U128 => PrimitiveKind::UnsignedInteger,
            I8 | I16 | I32 | I64 | I128 => PrimitiveKind::SignedInteger,
            F32 | F64 => PrimitiveKind::Float,
        }
    }

    pub fn default_value(&self) -> PrimitiveChannelValue {
        use PrimitiveChannelType::*;
        match self {
            U8 => PrimitiveChannelValue::U8(Default::default()),
            U16 => PrimitiveChannelValue::U16(Default::default()),
            U32 => PrimitiveChannelValue::U32(Default::default()),
            U64 => PrimitiveChannelValue::U64(Default::default()),
            U128 => PrimitiveChannelValue::U128(Default::default()),
            I8 => PrimitiveChannelValue::I8(Default::default()),
            I16 => PrimitiveChannelValue::I16(Default::default()),
            I32 => PrimitiveChannelValue::I32(Default::default()),
            I64 => PrimitiveChannelValue::I64(Default::default()),
            I128 => PrimitiveChannelValue::I128(Default::default()),
            F32 => PrimitiveChannelValue::F32(Default::default()),
            F64 => PrimitiveChannelValue::F64(Default::default()),
        }
    }

    pub fn parse(&self, from: impl AsRef<str>) -> Option<PrimitiveChannelValue> {
        use PrimitiveChannelType::*;
        Some(match self {
            U8 => PrimitiveChannelValue::U8(from.as_ref().parse().ok()?),
            U16 => PrimitiveChannelValue::U16(from.as_ref().parse().ok()?),
            U32 => PrimitiveChannelValue::U32(from.as_ref().parse().ok()?),
            U64 => PrimitiveChannelValue::U64(from.as_ref().parse().ok()?),
            U128 => PrimitiveChannelValue::U128(from.as_ref().parse().ok()?),
            I8 => PrimitiveChannelValue::I8(from.as_ref().parse().ok()?),
            I16 => PrimitiveChannelValue::I16(from.as_ref().parse().ok()?),
            I32 => PrimitiveChannelValue::I32(from.as_ref().parse().ok()?),
            I64 => PrimitiveChannelValue::I64(from.as_ref().parse().ok()?),
            I128 => PrimitiveChannelValue::I128(from.as_ref().parse().ok()?),
            F32 => PrimitiveChannelValue::F32(from.as_ref().parse().ok()?),
            F64 => PrimitiveChannelValue::F64(from.as_ref().parse().ok()?),
        })
    }

    pub fn read<E: ByteOrder, R>(&self, read: R) -> std::io::Result<PrimitiveChannelValue>
    where Cursor<R>: Read {
        use PrimitiveChannelType::*;
        let mut read = Cursor::new(read);
        Ok(match self {
            U8 => PrimitiveChannelValue::U8(read.read_u8()?),
            U16 => PrimitiveChannelValue::U16(read.read_u16::<E>()?),
            U32 => PrimitiveChannelValue::U32(read.read_u32::<E>()?),
            U64 => PrimitiveChannelValue::U64(read.read_u64::<E>()?),
            U128 => PrimitiveChannelValue::U128(read.read_u128::<E>()?),
            I8 => PrimitiveChannelValue::I8(read.read_i8()?),
            I16 => PrimitiveChannelValue::I16(read.read_i16::<E>()?),
            I32 => PrimitiveChannelValue::I32(read.read_i32::<E>()?),
            I64 => PrimitiveChannelValue::I64(read.read_i64::<E>()?),
            I128 => PrimitiveChannelValue::I128(read.read_i128::<E>()?),
            F32 => PrimitiveChannelValue::F32(read.read_f32::<E>()?),
            F64 => PrimitiveChannelValue::F64(read.read_f64::<E>()?),
        })
    }
}

impl Display for PrimitiveChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl ChannelTypeTrait for PrimitiveChannelType {
    fn value_size(&self) -> usize {
        use PrimitiveChannelType::*;
        match self {
            U8 | I8 => 1,
            U16 | I16 => 2,
            U32 | I32 | F32 => 4,
            U64 | I64 | F64 => 8,
            U128 | I128 => 16,
        }
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.kind().is_abi_compatible(&other.kind()) && self.value_size() == other.value_size()
    }
}

impl From<PrimitiveChannelType> for ChannelType {
    fn from(other: PrimitiveChannelType) -> Self {
        ChannelType::Primitive(other)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum OpaqueChannelType {
    Texture(TextureChannelType),
}

impl Display for OpaqueChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use OpaqueChannelType::*;
        match self {
            Texture(texture) => f.write_fmt(format_args!("{}", texture)),
        }
    }
}

#[repr(C)]
pub struct OpaqueValue {
    pub index: u32,
}

impl ChannelTypeTrait for OpaqueChannelType {
    fn value_size(&self) -> usize {
        std::mem::size_of::<OpaqueValue>()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl From<OpaqueChannelType> for ChannelType {
    fn from(other: OpaqueChannelType) -> Self {
        ChannelType::Opaque(other)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TextureChannelType {
    // TODO texture format, size?
}

impl Display for TextureChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Texture")
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ArrayChannelType {
    pub item_type: Box<ChannelType>,
    pub len: usize,
}

impl ArrayChannelType {
    pub fn new(item_type: impl Into<ChannelType>, len: usize) -> Self {
        Self { item_type: Box::new(item_type.into()), len }
    }

    pub fn single(item_type: impl Into<ChannelType>) -> Self {
        Self::new(item_type, 1)
    }
}

impl Display for ArrayChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}; {}]", self.item_type, self.len))
    }
}

impl ChannelTypeTrait for ArrayChannelType {
    fn value_size(&self) -> usize {
        self.len * self.item_type.value_size()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        if self.value_size() != other.value_size() {
            return false;
        }

        let mut item_type_a = &self.item_type;

        while let ChannelType::Array(array) = item_type_a.as_ref() {
            item_type_a = &array.item_type;
        }

        let mut item_type_b = &other.item_type;

        while let ChannelType::Array(array) = item_type_b.as_ref() {
            item_type_b = &array.item_type;
        }

        if let (ChannelType::Primitive(primitive_type_a), ChannelType::Primitive(primitive_type_b)) =
            (item_type_a.as_ref(), item_type_b.as_ref())
        {
            primitive_type_a.kind().is_abi_compatible(&primitive_type_b.kind())
        } else {
            item_type_a.is_abi_compatible(item_type_b)
        }
    }
}

impl From<ArrayChannelType> for ChannelType {
    fn from(other: ArrayChannelType) -> Self {
        ChannelType::Array(other)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Channel {
    pub title: String,
    pub description: Option<String>,
    pub ty: ChannelType,
}

impl Channel {
    pub fn new(title: impl ToString, ty: impl Into<ChannelType>) -> Self {
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
    pub ty: &'a ChannelType,
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
    pub fn zeroed(ty: &ChannelType) -> Self {
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

pub enum NodeCommand {
    Configure(NodeConfiguration),
}

pub trait NodeBehaviourMessage: Downcast + Debug + Send {
    fn dyn_clone(&self) -> Box<dyn NodeBehaviourMessage>;
}

impl_downcast!(NodeBehaviourMessage);

macro_rules! impl_node_behaviour_message {
    ($target_type:ident) => {
        impl NodeBehaviourMessage for $target_type {
            fn dyn_clone(&self) -> Box<dyn NodeBehaviourMessage> {
                Box::new(self.clone())
            }
        }
    };
}

impl Clone for Box<dyn NodeBehaviourMessage> {
    fn clone(&self) -> Self {
        NodeBehaviourMessage::dyn_clone(self.as_ref())
    }
}

#[derive(Debug, Clone)]
pub enum NodeEvent {
    Update,
    Message(Box<dyn NodeBehaviourMessage>),
}

// Should the state be cloneable?
// It should be serializable and deserializable, so probably cloneable as well?
// What about windows? Those should not be cloned?
pub trait NodeExecutorState: Downcast + DynClone + Debug + Send + Sync {}

impl<T> NodeExecutorState for T where T: Downcast + DynClone + Debug + Send + Sync {}

impl_downcast!(NodeExecutorState);

pub trait NodeExecutor: Send {
    fn execute(
        &self,
        state: Option<&mut dyn NodeExecutorState>,
        inputs: &ChannelValueRefs,
        outputs: &mut ChannelValues,
    );
}

impl<T> NodeExecutor for T
where T: Send + Fn(Option<&mut dyn NodeExecutorState>, &ChannelValueRefs, &mut ChannelValues)
{
    fn execute(
        &self,
        state: Option<&mut dyn NodeExecutorState>,
        inputs: &ChannelValueRefs,
        outputs: &mut ChannelValues,
    )
    {
        (self)(state, inputs, outputs)
    }
}

pub type ArcNodeExecutor = Arc<dyn NodeExecutor + Send + Sync>;

pub trait NodeBehaviour {
    fn name(&self) -> &str;
    fn update(&mut self, event: NodeEvent) -> Vec<NodeCommand>;
    fn view(&mut self, theme: &dyn Theme) -> Option<Element<Box<dyn NodeBehaviourMessage>>>;
    fn init_state(&self) -> Option<Box<dyn NodeExecutorState>>;
    fn create_executor(&self) -> ArcNodeExecutor;
}

pub mod binary_op;
pub mod constant;
pub mod counter;
