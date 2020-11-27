use std::any::TypeId;
use std::borrow::Cow;
use std::fmt::{Debug, Display};

pub use array::*;
use downcast_rs::Downcast;
pub use list::*;
pub use primitive::*;
pub use reference::*;
pub use texture::*;

pub mod array;
pub mod list;
pub mod primitive;
pub mod reference;
pub mod texture;

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
#[repr(transparent)]
pub struct AllocationPointer {
    pub(crate) index: u64,
}

pub trait TypeTrait: Into<TypeEnum> + Send + Sync + 'static {
    /// Returns the size of the associated value, in bytes.
    fn value_size(&self) -> usize;

    /// Returns `true`, if the type can be safely cast/reinterpreted as another.
    /// Otherwise returns `false`.
    fn is_abi_compatible(&self, other: &Self) -> bool;

    /// Returns `true`, whether it is possible to let the user read the underlying
    /// binary representation of the associated value. Otherwise returns `false`.
    ///
    /// Pointers are the typical case which is not safe.
    fn has_safe_binary_representation(&self) -> bool;
}

pub trait DowncastFromTypeEnum {
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self>;
    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self>;
}

pub trait InitializeWith<T>: Default {
    fn initialize_with(&mut self, descriptor: T);
}

impl InitializeWith<()> for () {
    fn initialize_with(&mut self, descriptor: ()) {}
}

pub trait DynTypeDescriptor<T: DynTypeTrait<Descriptor = Self>>: Send + Sync + 'static {
    fn get_type(&self) -> T;
}

/// A type that can only be created on the heap.
pub trait DynTypeTrait: Into<TypeEnum> + Send + Sync + 'static {
    // /// The interface to access the dynamically allocated data.
    // /// Abstracts away the underlying implementation.
    // type DynAllocDispatcher: Send + Sync + 'static;

    /// The type to initialize the allocation with.
    type Descriptor: DynTypeDescriptor<Self>;
}

impl<T> TypeTrait for T
where T: DynTypeTrait
{
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        true
    }

    fn has_safe_binary_representation(&self) -> bool {
        false
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum TypeEnum {
    Primitive(PrimitiveType),
    // Tuple(Vec<Self>),
    Array(ArrayType),
    List(ListType),
    Texture(TextureType),
}

impl TypeEnum {
    pub fn downcast_ref<T: DowncastFromTypeEnum>(&self) -> Option<&T> {
        T::downcast_from_ref(self)
    }

    pub fn downcast_mut<T: DowncastFromTypeEnum>(&mut self) -> Option<&mut T> {
        T::downcast_from_mut(self)
    }
}

impl Display for TypeEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TypeEnum::*;
        match self {
            Primitive(primitive) => f.write_fmt(format_args!("{}", primitive)),
            Array(array) => f.write_fmt(format_args!("{}", array)),
            List(list) => f.write_fmt(format_args!("{}", list)),
            Texture(texture) => f.write_fmt(format_args!("{}", texture)),
        }
    }
}

impl TypeTrait for TypeEnum {
    fn value_size(&self) -> usize {
        use TypeEnum::*;
        match self {
            Primitive(primitive) => primitive.value_size(),
            Array(array) => array.value_size(),
            List(list) => list.value_size(),
            Texture(texture) => texture.value_size(),
        }
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        use TypeEnum::*;
        match (self, other) {
            (Primitive(a), Primitive(b)) => return a.is_abi_compatible(b),
            (Texture(a), Texture(b)) => return a.is_abi_compatible(b),
            _ => (),
        }
        if matches!(self, Array { .. }) || matches!(other, Array { .. }) {
            let a = if let Array(array) = self {
                Cow::Borrowed(array)
            } else {
                Cow::Owned(ArrayType::single(self.clone()))
            };
            let b = if let Array(array) = other {
                Cow::Borrowed(array)
            } else {
                Cow::Owned(ArrayType::single(other.clone()))
            };
            return a.is_abi_compatible(&b);
        }

        false
    }

    fn has_safe_binary_representation(&self) -> bool {
        use TypeEnum::*;
        match self {
            Primitive(primitive) => primitive.has_safe_binary_representation(),
            Array(array) => array.has_safe_binary_representation(),
            List(list) => list.has_safe_binary_representation(),
            Texture(texture) => texture.has_safe_binary_representation(),
        }
    }
}
