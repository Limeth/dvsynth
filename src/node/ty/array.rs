use std::borrow::Cow;
use std::fmt::Display;

use super::{Bytes, DowncastFromTypeEnum, SizedTypeExt, TypeEnum, TypeExt, TypedBytes};

pub mod prelude {}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ArrayType {
    pub item_type: Box<TypeEnum>,
    pub len: usize,
}

impl ArrayType {
    pub fn new(item_type: impl Into<TypeEnum> + SizedTypeExt, len: usize) -> Self {
        Self { item_type: Box::new(item_type.into()), len }
    }

    pub fn single(item_type: impl Into<TypeEnum> + SizedTypeExt) -> Self {
        Self::new(item_type, 1)
    }

    pub fn new_if_sized(item_type: impl Into<TypeEnum>, len: usize) -> Option<Self> {
        let item_type = item_type.into();
        item_type.value_size_if_sized().map(|_| Self { item_type: Box::new(item_type), len })
    }

    pub fn single_if_sized(item_type: impl Into<TypeEnum>) -> Option<Self> {
        Self::new_if_sized(item_type, 1)
    }
}

impl Display for ArrayType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("[{}; {}]", self.item_type, self.len))
    }
}

unsafe impl SizedTypeExt for ArrayType {
    fn value_size(&self) -> usize {
        self.len * self.item_type.value_size_if_sized().unwrap()
    }
}

unsafe impl TypeExt for ArrayType {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        if self.value_size() != other.value_size() {
            return false;
        }

        let mut item_type_a = &self.item_type;

        while let TypeEnum::Array(array) = item_type_a.as_ref() {
            item_type_a = &array.item_type;
        }

        let mut item_type_b = &other.item_type;

        while let TypeEnum::Array(array) = item_type_b.as_ref() {
            item_type_b = &array.item_type;
        }

        if let (TypeEnum::Primitive(primitive_type_a), TypeEnum::Primitive(primitive_type_b)) =
            (item_type_a.as_ref(), item_type_b.as_ref())
        {
            primitive_type_a.kind().is_abi_compatible(&primitive_type_b.kind())
        } else {
            item_type_a.is_abi_compatible(item_type_b)
        }
    }

    unsafe fn children<'a>(&'a self, data: Bytes<'a>) -> Vec<TypedBytes<'a>> {
        let value_size = self.item_type.value_size_if_sized().unwrap();
        let bytes = data.bytes().unwrap();

        bytes
            .chunks_exact(value_size)
            .map(|chunk| TypedBytes::from(chunk, Cow::Borrowed(self.item_type.as_ref())))
            .collect()
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn has_safe_binary_representation(&self) -> bool {
        self.item_type.has_safe_binary_representation()
    }

    fn is_cloneable(&self) -> bool {
        self.item_type.is_cloneable()
    }
}

impl From<ArrayType> for TypeEnum {
    fn from(other: ArrayType) -> Self {
        TypeEnum::Array(other)
    }
}

impl_downcast_from_type_enum!(Array(ArrayType));
