use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::Cow;
use std::fmt::Display;
use std::io::Cursor;
use std::ops::{Add, Deref, DerefMut, Div, Index, IndexMut, Mul, Sub};

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
    pub fn kind(&self) -> PrimitiveKind {
        use PrimitiveChannelType::*;
        match self {
            U8 | U16 | U32 | U64 | U128 => PrimitiveKind::UnsignedInteger,
            I8 | I16 | I32 | I64 | I128 => PrimitiveKind::SignedInteger,
            F32 | F64 => PrimitiveKind::Float,
        }
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

#[derive(Debug, Clone)]
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

pub trait NodeBehaviour {
    fn name(&self) -> &str;
    fn update(&mut self) -> NodeConfiguration;
    fn execute(&self, inputs: &ChannelValues, outputs: &mut ChannelValues);
}

pub struct ConstantNodeBehaviour {
    pub value: u32,
}

impl NodeBehaviour for ConstantNodeBehaviour {
    fn name(&self) -> &str {
        "Constant"
    }

    fn update(&mut self) -> NodeConfiguration {
        NodeConfiguration {
            channels_input: Vec::new(),
            channels_output: vec![Channel::new("value", PrimitiveChannelType::U32)],
        }
    }

    fn execute(&self, _inputs: &ChannelValues, outputs: &mut ChannelValues) {
        let mut cursor = Cursor::new(outputs[0].as_mut());

        cursor.write_u32::<LittleEndian>(self.value).unwrap();
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

impl BinaryOp {
    pub fn apply<T>(self, lhs: T, rhs: T) -> T
    where T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> {
        match self {
            BinaryOp::Add => Add::add(lhs, rhs),
            BinaryOp::Sub => Sub::sub(lhs, rhs),
            BinaryOp::Mul => Mul::mul(lhs, rhs),
            BinaryOp::Div => Div::div(lhs, rhs),
        }
    }
}

pub struct BinaryOpNodeBehaviour {
    pub op: BinaryOp,
}

impl NodeBehaviour for BinaryOpNodeBehaviour {
    fn name(&self) -> &str {
        "Binary Operation"
    }

    fn update(&mut self) -> NodeConfiguration {
        NodeConfiguration {
            channels_input: vec![
                Channel::new("lhs", PrimitiveChannelType::U32),
                Channel::new("rhs", PrimitiveChannelType::U32),
            ],
            channels_output: vec![Channel::new("result", PrimitiveChannelType::U32)],
        }
    }

    fn execute(&self, inputs: &ChannelValues, outputs: &mut ChannelValues) {
        let lhs: u32 = inputs[0].as_ref().read_u32::<LittleEndian>().unwrap();
        let rhs: u32 = inputs[1].as_ref().read_u32::<LittleEndian>().unwrap();
        let result = self.op.apply(lhs, rhs);
        let mut output_cursor = Cursor::new(outputs[0].as_mut());

        dbg!(result);

        output_cursor.write_u32::<LittleEndian>(result).unwrap();
    }
}

// pub struct TestNodeBehaviour {
//     pub name: String,
//     pub channels_input: Vec<ChannelType>,
//     pub channels_output: Vec<ChannelType>,
// }

// impl NodeBehaviour for TestNodeBehaviour {
//     fn name(&self) -> &str {
//         &self.name
//     }

//     fn update(&mut self) -> NodeConfiguration {
//         NodeConfiguration {
//             channels_input: self
//                 .channels_input
//                 .iter()
//                 .cloned()
//                 .enumerate()
//                 .map(|(i, ty)| Channel { title: format!("In {}: {}", i, ty), description: None, ty })
//                 .collect(),
//             channels_output: self
//                 .channels_output
//                 .iter()
//                 .cloned()
//                 .enumerate()
//                 .map(|(i, ty)| Channel { title: format!("Out {}: {}", i, ty), description: None, ty })
//                 .collect(),
//         }
//     }

//     fn execute(&self, inputs: &ChannelValues, outputs: &mut ChannelValues) {

//     }
// }
