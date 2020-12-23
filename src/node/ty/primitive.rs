use super::{
    Bytes, CloneableTypeExt, DowncastFromTypeEnum, SafeBinaryRepresentationTypeExt, SizedTypeExt, TypeEnum,
    TypeExt, TypedBytes,
};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use std::fmt::Display;
use std::io::{Cursor, Read, Write};

pub mod prelude {}

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
    pub fn ty(&self) -> PrimitiveType {
        use PrimitiveChannelValue::*;
        match self {
            U8(_) => PrimitiveType::U8,
            U16(_) => PrimitiveType::U16,
            U32(_) => PrimitiveType::U32,
            U64(_) => PrimitiveType::U64,
            U128(_) => PrimitiveType::U128,
            I8(_) => PrimitiveType::I8,
            I16(_) => PrimitiveType::I16,
            I32(_) => PrimitiveType::I32,
            I64(_) => PrimitiveType::I64,
            I128(_) => PrimitiveType::I128,
            F32(_) => PrimitiveType::F32,
            F64(_) => PrimitiveType::F64,
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
pub enum PrimitiveType {
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

impl PrimitiveType {
    pub const VALUES: [PrimitiveType; 12] = {
        use PrimitiveType::*;
        [U8, U16, U32, U64, U128, I8, I16, I32, I64, I128, F32, F64]
    };

    pub fn kind(&self) -> PrimitiveKind {
        use PrimitiveType::*;
        match self {
            U8 | U16 | U32 | U64 | U128 => PrimitiveKind::UnsignedInteger,
            I8 | I16 | I32 | I64 | I128 => PrimitiveKind::SignedInteger,
            F32 | F64 => PrimitiveKind::Float,
        }
    }

    pub fn default_value(&self) -> PrimitiveChannelValue {
        use PrimitiveType::*;
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
        use PrimitiveType::*;
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
        use PrimitiveType::*;
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

impl Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

unsafe impl SizedTypeExt for PrimitiveType {
    fn value_size(&self) -> usize {
        use PrimitiveType::*;
        match self {
            U8 | I8 => 1,
            U16 | I16 => 2,
            U32 | I32 | F32 => 4,
            U64 | I64 | F64 => 8,
            U128 | I128 => 16,
        }
    }
}

unsafe impl SafeBinaryRepresentationTypeExt for PrimitiveType {}

unsafe impl CloneableTypeExt for PrimitiveType {}

unsafe impl TypeExt for PrimitiveType {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.kind().is_abi_compatible(&other.kind()) && self.value_size() == other.value_size()
    }

    unsafe fn children<'a>(&'a self, _data: Bytes<'a>) -> Vec<TypedBytes<'a>> {
        vec![]
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn has_safe_binary_representation(&self) -> bool {
        true
    }

    fn is_cloneable(&self) -> bool {
        true
    }
}

// trait PrimitiveRefExt {
//     pub
// }

// impl<R> R where R: RefExt<PrimitiveType> {}

impl From<PrimitiveType> for TypeEnum {
    fn from(other: PrimitiveType) -> Self {
        TypeEnum::Primitive(other)
    }
}

impl_downcast_from_type_enum!(Primitive(PrimitiveType));
