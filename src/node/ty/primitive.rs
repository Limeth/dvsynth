use super::{
    Bytes, CloneableTypeExt, DowncastFromTypeEnum, SafeBinaryRepresentationTypeExt, SizedTypeExt, TypeDesc,
    TypeEnum, TypeExt, TypeResolution, TypeTrait, TypedBytes,
};
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};
use std::any::TypeId;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;

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

macro_rules! impl_primitive_types {
    {
        $($enum_variant:ident ($primitive_type:ident, $primitive_kind:ident)),*$(,)?
    } => {
        pub struct PrimitiveType<T: 'static> {
            __marker: PhantomData<T>,
        }

        impl<T> Hash for PrimitiveType<T> {
            fn hash<H>(&self, state: &mut H)
            where
                H: Hasher
            {
                TypeId::of::<T>().hash(state);
            }
        }

        impl<T> Clone for PrimitiveType<T> {
            fn clone(&self) -> Self {
                Self { __marker: Default::default() }
            }
        }

        impl<T> Default for PrimitiveType<T> {
            fn default() -> Self {
                Self { __marker: Default::default() }
            }
        }

        impl<T> PartialEq for PrimitiveType<T> {
            fn eq(&self, other: &Self) -> bool {
                true
            }
        }

        impl<T> Eq for PrimitiveType<T> {}

        impl<T> Display for PrimitiveType<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("PrimitiveType<{}>", std::any::type_name::<T>()))
            }
        }

        impl<T> Debug for PrimitiveType<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("PrimitiveType<{}>", std::any::type_name::<T>()))
            }
        }

        #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
        pub enum PrimitiveTypeEnum {
            $($enum_variant,)*
        }

        impl From<PrimitiveTypeEnum> for TypeEnum {
            fn from(from: PrimitiveTypeEnum) -> Self {
                use PrimitiveTypeEnum::*;
                match from {
                    $(
                        $enum_variant => TypeEnum::$enum_variant(Default::default()),
                    )*
                }
            }
        }

        /// Should not be used for large data storage, as the size is defined by the largest variant.
        #[derive(Debug, PartialEq, Clone, Copy)]
        pub enum PrimitiveChannelValue {
            $($enum_variant($primitive_type),)*
        }

        impl PrimitiveTypeEnum {
            pub const VALUES: [PrimitiveTypeEnum; 12] = {
                use PrimitiveTypeEnum::*;
                [$($enum_variant,)*]
            };

            pub fn kind(&self) -> PrimitiveKind {
                use PrimitiveTypeEnum::*;
                match self {
                    $(
                        $enum_variant => PrimitiveKind::$primitive_kind,
                    )*
                }
            }

            pub fn default_value(&self) -> PrimitiveChannelValue {
                use PrimitiveTypeEnum::*;
                match self {
                    $(
                        $enum_variant => PrimitiveChannelValue::$enum_variant(Default::default()),
                    )*
                }
            }

            pub fn parse(&self, from: impl AsRef<str>) -> Option<PrimitiveChannelValue> {
                use PrimitiveTypeEnum::*;
                Some(match self {
                    $(
                        $enum_variant => PrimitiveChannelValue::$enum_variant(from.as_ref().parse().ok()?),
                    )*
                })
            }
        }

        impl PrimitiveChannelValue {
            pub fn ty(&self) -> PrimitiveTypeEnum {
                use PrimitiveChannelValue::*;
                match self {
                    $(
                        $enum_variant(_) => PrimitiveTypeEnum::$enum_variant,
                    )*
                }
            }

            pub fn value_to_string(&self) -> String {
                use PrimitiveChannelValue::*;
                match self {
                    $(
                        $enum_variant(value) => value.to_string(),
                    )*
                }
            }
        }

        $(
            impl From<$primitive_type> for PrimitiveChannelValue {
                fn from(primitive: $primitive_type) -> Self {
                    PrimitiveChannelValue::$enum_variant(primitive)
                }
            }

            unsafe impl SizedTypeExt for PrimitiveType<$primitive_type> {
                fn value_size(&self) -> usize {
                    std::mem::size_of::<$primitive_type>()
                }
            }

            unsafe impl SafeBinaryRepresentationTypeExt for PrimitiveType<$primitive_type> {}

            unsafe impl CloneableTypeExt for PrimitiveType<$primitive_type> {}

            unsafe impl TypeExt for PrimitiveType<$primitive_type> {
                fn is_abi_compatible(&self, other: &Self) -> bool {
                    true
                }

                unsafe fn children<'a>(&'a self, _data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
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

            unsafe impl TypeDesc for PrimitiveType<$primitive_type> {}
            impl TypeTrait for PrimitiveType<$primitive_type> {}

            impl From<PrimitiveType<$primitive_type>> for TypeEnum {
                fn from(from: PrimitiveType<$primitive_type>) -> Self {
                    TypeEnum::$enum_variant(from)
                }
            }

            // impl DowncastFromTypeEnum for PrimitiveType<$primitive_type> {
            //     fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
            //     where Self: Sized {
            //         if let TypeEnum::$enum_variant(value) = from {
            //             Some(value)
            //         } else {
            //             None
            //         }
            //     }

            //     fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>> {
            //         if let TypeEnum::$enum_variant(value) = from {
            //             Some(value)
            //         } else {
            //             None
            //         }
            //     }

            //     fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>> {
            //         if let TypeEnum::$enum_variant(value) = from {
            //             Some(value)
            //         } else {
            //             None
            //         }
            //     }
            // }

            impl_downcast_from_type_enum!($enum_variant(PrimitiveType<$primitive_type>));
        )*
    }
}

impl_primitive_types! {
    U8(u8, UnsignedInteger),
    U16(u16, UnsignedInteger),
    U32(u32, UnsignedInteger),
    U64(u64, UnsignedInteger),
    U128(u128, UnsignedInteger),
    I8(i8, SignedInteger),
    I16(i16, SignedInteger),
    I32(i32, SignedInteger),
    I64(i64, SignedInteger),
    I128(i128, SignedInteger),
    F32(f32, Float),
    F64(f64, Float),
}

impl PrimitiveTypeEnum {
    pub fn read<E: ByteOrder, R>(&self, read: R) -> std::io::Result<PrimitiveChannelValue>
    where Cursor<R>: Read {
        use PrimitiveTypeEnum::*;
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

impl Display for PrimitiveTypeEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl PrimitiveChannelValue {
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
