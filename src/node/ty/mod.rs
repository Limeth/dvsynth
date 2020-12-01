use std::any::TypeId;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use downcast_rs::Downcast;

pub use array::*;
pub use list::*;
pub use primitive::*;
pub use ptr::*;
pub use reference::*;
pub use texture::*;

use crate::graph::alloc::{AllocatedType, Allocator};

use super::behaviour::AllocatorHandle;

pub mod array;
pub mod list;
pub mod primitive;
pub mod ptr;
pub mod reference;
pub mod texture;

pub mod prelude {
    pub use super::reference::{RefDynExt, RefExt, RefMutDynExt, RefMutExt};
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
#[repr(transparent)]
pub struct AllocationPointer {
    pub(crate) index: u64,
}

// pub trait InnerRef<'a>: Sized + Clone + Copy {
//     type OutputData: ?Sized;
//     type OutputType: TypeTrait;

//     unsafe fn from_raw_bytes(
//         bytes: &'a [u8],
//         ty: &'a Self::OutputType,
//         handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>;

//     unsafe fn raw_bytes(&self) -> &[u8];
//     unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()>;
// }

// pub trait InnerRefMut<'a>: Sized {
//     type OutputData: ?Sized;
//     type OutputType: TypeTrait;
//     type InnerRef: self::InnerRef<'a>;

//     unsafe fn from_raw_bytes(
//         bytes: &'a mut [u8],
//         ty: &'a Self::OutputType,
//         handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>;

//     unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()>;
// }

// pub trait InnerRefTypes<T: TypeTrait> {
//     type InnerRef<'a>: self::InnerRef<'a, OutputType = T>;
//     type InnerRefMut<'a>: self::InnerRefMut<'a, OutputType = T>;

//     fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a>;
// }

#[derive(Clone, Copy)]
pub enum Bytes<'a> {
    Bytes(&'a [u8]),
    Object(&'a dyn AllocatedType),
}

impl<'a> Bytes<'a> {
    pub fn bytes(self) -> Option<&'a [u8]> {
        if let Bytes::Bytes(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn object(self) -> Option<&'a dyn AllocatedType> {
        if let Bytes::Object(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
}

impl<'a> From<&'a [u8]> for Bytes<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Bytes::Bytes(bytes)
    }
}

impl<'a> From<&'a dyn AllocatedType> for Bytes<'a> {
    fn from(val: &'a dyn AllocatedType) -> Self {
        Bytes::Object(val)
    }
}

impl<'a> From<BytesMut<'a>> for Bytes<'a> {
    fn from(bytes: BytesMut<'a>) -> Self {
        bytes.downgrade()
    }
}

#[derive(Clone, Copy)]
pub struct TypedBytes<'a> {
    bytes: Bytes<'a>,
    ty: &'a TypeEnum,
}

impl<'a> TypedBytes<'a> {
    pub fn from(bytes: impl Into<Bytes<'a>>, ty: &'a TypeEnum) -> Self {
        Self { bytes: bytes.into(), ty }
    }

    pub fn bytes(self) -> Bytes<'a> {
        self.bytes
    }

    pub fn ty(self) -> &'a TypeEnum {
        self.ty
    }
}

pub enum BytesMut<'a> {
    Bytes(&'a mut [u8]),
    Object(&'a mut dyn AllocatedType),
}

impl<'a> BytesMut<'a> {
    pub fn downgrade(self) -> Bytes<'a> {
        match self {
            BytesMut::Bytes(inner) => Bytes::Bytes(&*inner),
            BytesMut::Object(inner) => Bytes::Object(&*inner),
        }
    }

    pub fn bytes(self) -> Option<&'a [u8]> {
        if let BytesMut::Bytes(inner) = self {
            Some(&*inner)
        } else {
            None
        }
    }

    pub fn object(self) -> Option<&'a dyn AllocatedType> {
        if let BytesMut::Object(inner) = self {
            Some(&*inner)
        } else {
            None
        }
    }

    pub fn bytes_mut(self) -> Option<&'a mut [u8]> {
        if let BytesMut::Bytes(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn object_mut(self) -> Option<&'a mut dyn AllocatedType> {
        if let BytesMut::Object(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
}

impl<'a> From<&'a mut [u8]> for BytesMut<'a> {
    fn from(bytes: &'a mut [u8]) -> Self {
        BytesMut::Bytes(bytes)
    }
}

impl<'a> From<&'a mut dyn AllocatedType> for BytesMut<'a> {
    fn from(val: &'a mut dyn AllocatedType) -> Self {
        BytesMut::Object(val)
    }
}

pub struct TypedBytesMut<'a> {
    bytes: BytesMut<'a>,
    ty: &'a TypeEnum,
}

impl<'a> TypedBytesMut<'a> {
    pub fn from(bytes: impl Into<BytesMut<'a>>, ty: &'a TypeEnum) -> Self {
        Self { bytes: bytes.into(), ty }
    }

    pub fn bytes_mut(self) -> BytesMut<'a> {
        self.bytes
    }

    pub fn bytes(self) -> Bytes<'a> {
        self.bytes.downgrade()
    }

    pub fn ty(self) -> &'a TypeEnum {
        self.ty
    }

    pub fn downgrade(self) -> TypedBytes<'a> {
        TypedBytes::from(self.bytes, self.ty)
    }
}

impl<'a> From<TypedBytesMut<'a>> for TypedBytes<'a> {
    fn from(from: TypedBytesMut<'a>) -> Self {
        from.downgrade()
    }
}

pub struct DirectInnerRefTypes<T> {
    __marker: PhantomData<T>,
}

pub trait TypeExt: Into<TypeEnum> + PartialEq + Eq + Send + Sync + 'static {
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

pub trait TypeTrait: TypeExt + DowncastFromTypeEnum {}

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
pub trait DynTypeTrait
where Self: TypeTrait
{
    /// The type to initialize the allocation with.
    type Descriptor: DynTypeDescriptor<Self>;
    type DynAlloc: AllocatedType;

    fn create_value_from_descriptor(descriptor: Self::Descriptor) -> Self::DynAlloc;
}

// pub struct IndirectInnerRef<'a, T: AllocatedType> {
//     pub(crate) ptr: AllocationPointer,
//     pub(crate) ty: &'a T,
//     __marker: PhantomData<T>,
// }

// impl<'a, T: DynTypeTrait> IndirectInnerRef<'a, T> {
//     pub fn new(ptr: AllocationPointer) -> Self {
//         let (_, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };
//         let ty = ty.downcast_ref().unwrap();
//         Self { ptr, ty, __marker: Default::default() }
//     }
// }

// impl<'a, T: DynTypeTrait> Clone for IndirectInnerRef<'a, T> {
//     fn clone(&self) -> Self {
//         Self { ptr: self.ptr, ty: self.ty, __marker: Default::default() }
//     }
// }

// impl<'a, T: DynTypeTrait> Copy for IndirectInnerRef<'a, T> {}

// impl<'a, T: DynTypeTrait> InnerRef<'a> for IndirectInnerRef<'a, T> {
//     type OutputData = T::DynAlloc;
//     type OutputType = T;

//     unsafe fn from_raw_bytes(
//         bytes: &'a [u8],
//         ty: &'a Self::OutputType,
//         _handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>
//     {
//         if bytes.len() != std::mem::size_of::<AllocationPointer>() {
//             return Err(());
//         }
//         let mut read = Cursor::new(bytes);
//         let ptr = AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap());
//         Ok(Self { ptr, ty, __marker: Default::default() })
//     }

//     unsafe fn raw_bytes(&self) -> &[u8] {
//         safe_transmute::transmute_to_bytes(&[self.ptr.as_u64()])
//     }

//     unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()> {
//         let (data, ty) = Allocator::get().deref_ptr(self.ptr).unwrap();
//         let ty = ty.downcast_ref().ok_or(())?;

//         if ty != self.ty {
//             return Err(());
//         }

//         Ok((data.downcast_ref().ok_or(())?, ty))
//     }
// }

// pub struct IndirectInnerRefMut<'a, T: AllocatedType> {
//     pub(crate) ptr: AllocationPointer,
//     pub(crate) ty: &'a T,
//     __marker: PhantomData<T>,
// }

// impl<'a, T: DynTypeTrait> IndirectInnerRefMut<'a, T> {
//     pub fn new(ptr: AllocationPointer) -> Self {
//         let (_, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };
//         let ty = ty.downcast_ref().unwrap();
//         Self { ptr, ty, __marker: Default::default() }
//     }
// }

// impl<'a, T> InnerRefMut<'a> for IndirectInnerRefMut<'a, T>
// where T: DynTypeTrait
// {
//     type OutputData = T::DynAlloc;
//     type OutputType = T;
//     type InnerRef = IndirectInnerRef<'a, T>;

//     unsafe fn from_raw_bytes(
//         bytes: &'a mut [u8],
//         ty: &'a Self::OutputType,
//         _handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>
//     {
//         if bytes.len() != std::mem::size_of::<AllocationPointer>() {
//             return Err(());
//         }
//         let mut read = Cursor::new(bytes);
//         let ptr = AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap());
//         Ok(Self { ptr, ty, __marker: Default::default() })
//     }

//     unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()> {
//         let (data, ty) = Allocator::get().deref_mut_ptr(self.ptr).unwrap();
//         let ty = ty.downcast_ref().ok_or(())?;

//         if ty != self.ty {
//             return Err(());
//         }

//         Ok((data.downcast_mut().ok_or(())?, ty))
//     }
// }

// pub struct IndirectInnerRefTypes<T> {
//     __marker: PhantomData<T>,
// }

// impl<T: DynTypeTrait> InnerRefTypes<T> for IndirectInnerRefTypes<T> {
//     type InnerRef<'a> = IndirectInnerRef<'a, T>;
//     type InnerRefMut<'a> = IndirectInnerRefMut<'a, T>;

//     fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a> {
//         IndirectInnerRef { ptr: from.ptr, ty: from.ty, __marker: Default::default() }
//     }
// }

impl<T> TypeTrait for T where T: DynTypeTrait {}

impl<T> TypeExt for T
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

pub trait SizedType: TypeTrait {}

macro_rules! define_type_enum {
    [
        $($variant:ident($inner:ident)),*$(,)?
    ] => {
        #[derive(Debug, Hash, PartialEq, Eq, Clone)]
        pub enum TypeEnum {
            $(
                $variant($inner),
            )*
        }

        impl Display for TypeEnum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => f.write_fmt(format_args!("{}", inner)),
                    )*
                }
            }
        }

        impl TypeEnum {
            fn value_size_impl(&self) -> usize {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => inner.value_size(),
                    )*
                }
            }

            fn has_safe_binary_representation_impl(&self) -> bool {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => inner.has_safe_binary_representation(),
                    )*
                }
            }
        }
    }
}

define_type_enum![
    Shared(Shared),
    Unique(Unique),
    Primitive(PrimitiveType),
    // Tuple(Vec<Self>),
    Array(ArrayType),
    List(ListType),
    Texture(TextureType),
];

impl TypeEnum {
    pub fn downcast_ref<T: DowncastFromTypeEnum>(&self) -> Option<&T> {
        T::downcast_from_ref(self)
    }

    pub fn downcast_mut<T: DowncastFromTypeEnum>(&mut self) -> Option<&mut T> {
        T::downcast_from_mut(self)
    }
}

impl TypeExt for TypeEnum {
    fn value_size(&self) -> usize {
        self.value_size_impl()
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
        self.has_safe_binary_representation_impl()
    }
}
