use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Display;
use std::io::{Cursor, Read, Write};

use crate::graph::alloc::Allocator;

use super::{
    AllocationPointer, DowncastFromTypeEnum, RefAny, RefExt, RefMutAny, RefMutExt, TypeEnum, TypeExt,
    TypeTrait, TypedBytes, TypedBytesMut,
};

pub unsafe trait SharedTrait: TypeTrait {}
pub unsafe trait UniqueTrait: SharedTrait {}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Unique {
    pub child_ty: Box<TypeEnum>,
}

impl Unique {
    pub fn new(child_ty: Box<TypeEnum>) -> Self {
        Self { child_ty }
    }
}

impl Display for Unique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Unique<{}>", self.child_ty))
    }
}

impl TypeExt for Unique {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    fn has_safe_binary_representation(&self) -> bool {
        true
    }
}

impl From<Unique> for TypeEnum {
    fn from(other: Unique) -> Self {
        TypeEnum::Unique(other)
    }
}

impl DowncastFromTypeEnum for Unique {
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self> {
        if let TypeEnum::Unique(inner) = from {
            Some(inner)
        } else {
            None
        }
    }

    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self> {
        if let TypeEnum::Unique(inner) = from {
            Some(inner)
        } else {
            None
        }
    }
}

impl TypeTrait for Unique {}
unsafe impl SharedTrait for Unique {}
unsafe impl UniqueTrait for Unique {}

pub trait UniqueRefExt<'a> {
    fn deref(self) -> RefAny<'a>;
}

pub trait UniqueRefMutExt<'a> {
    fn deref_mut(self) -> RefMutAny<'a>;
}

impl<'a, T> UniqueRefMutExt<'a> for T
where T: RefMutExt<'a, Unique>
{
    fn deref_mut(self) -> RefMutAny<'a> {
        // FIXME check types?

        let typed_bytes = unsafe { self.typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        let (data, ty) = unsafe { Allocator::get().deref_mut_ptr(ptr).unwrap() };

        unsafe { RefMutAny::from(TypedBytesMut::from(data, ty)) }
    }
}

impl<'a, T> UniqueRefExt<'a> for T
where T: RefExt<'a, Unique>
{
    fn deref(self) -> RefAny<'a> {
        // FIXME check types?

        let typed_bytes = unsafe { self.typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        let (data, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };

        unsafe { RefAny::from(TypedBytes::from(data, ty)) }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Shared {
    pub child_ty: Box<TypeEnum>,
}

impl Shared {
    pub fn new(child_ty: Box<TypeEnum>) -> Self {
        Self { child_ty }
    }
}

impl Display for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Shared<{}>", self.child_ty))
    }
}

impl TypeExt for Shared {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    fn has_safe_binary_representation(&self) -> bool {
        true
    }
}

impl From<Shared> for TypeEnum {
    fn from(other: Shared) -> Self {
        TypeEnum::Shared(other)
    }
}

impl DowncastFromTypeEnum for Shared {
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self> {
        if let TypeEnum::Shared(inner) = from {
            Some(inner)
        } else {
            None
        }
    }

    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self> {
        if let TypeEnum::Shared(inner) = from {
            Some(inner)
        } else {
            None
        }
    }
}

impl TypeTrait for Shared {}
unsafe impl SharedTrait for Shared {}

pub trait SharedRefExt<'a> {
    fn deref(self) -> RefAny<'a>;
}

pub trait SharedRefMutExt<'a> {}

impl<'a, T> SharedRefExt<'a> for T
where T: RefExt<'a, Shared>
{
    fn deref(self) -> RefAny<'a> {
        // FIXME check types?

        let typed_bytes = unsafe { self.typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        let (data, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };

        unsafe { RefAny::from(TypedBytes::from(data, ty)) }
    }
}

impl<'a, T> SharedRefMutExt<'a> for T where T: RefMutExt<'a, Shared> {}
