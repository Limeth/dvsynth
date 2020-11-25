use super::{DowncastFromTypeEnum, DynTypeTrait, TypeEnum};
use std::fmt::Display;

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TextureType {
    // TODO texture format, size?
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

impl DynTypeTrait for TextureType {
    // type DynAllocDispatcher = TextureDispatcher;
    type Descriptor = TextureDescriptor;
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
