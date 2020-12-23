use crate::graph::TextureAllocation;

use super::{Bytes, DowncastFromTypeEnum, DynTypeDescriptor, DynTypeTrait, TypeEnum, TypedBytes};
use std::fmt::Display;

pub mod prelude {}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TextureType {
    // TODO texture format, size?
}

impl TextureType {
    pub fn new() -> Self {
        Self {}
    }
}

impl Display for TextureType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Texture")
    }
}

impl From<TextureType> for TypeEnum {
    fn from(other: TextureType) -> Self {
        TypeEnum::Texture(other).into()
    }
}

// TODO
// pub struct TextureDispatcher;
pub struct TextureDescriptor;

impl DynTypeDescriptor<TextureType> for TextureDescriptor {
    fn get_type(&self) -> TextureType {
        TextureType {}
    }
}

impl DynTypeTrait for TextureType {
    // type DynAllocDispatcher = TextureDispatcher;
    type Descriptor = TextureDescriptor;
    type DynAlloc = TextureAllocation;

    fn create_value_from_descriptor(_descriptor: Self::Descriptor) -> Self::DynAlloc {
        todo!()
    }

    fn is_abi_compatible(&self, _other: &Self) -> bool {
        todo!()
    }

    unsafe fn children<'a>(&'a self, _data: &Bytes<'a>) -> Vec<TypedBytes<'a>> {
        todo!()
    }
}

impl DowncastFromTypeEnum for TextureType {
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self> {
        if let TypeEnum::Texture(inner) = from {
            Some(inner)
        } else {
            None
        }
    }

    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self> {
        if let TypeEnum::Texture(inner) = from {
            Some(inner)
        } else {
            None
        }
    }
}
