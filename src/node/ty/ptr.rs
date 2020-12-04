use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};
use std::fmt::Display;
use std::io::{Cursor, Read, Write};

use crate::graph::alloc::Allocator;
use crate::node::behaviour::AllocatorHandle;

use super::{
    AllocationPointer, DowncastFromTypeEnum, OwnedRefMut, RefAny, RefDynExt, RefExt, RefMut, RefMutAny,
    RefMutDynExt, RefMutExt, SizedTypeExt, TypeEnum, TypeExt, TypeTrait, TypedBytes, TypedBytesMut,
};

pub mod prelude {
    pub use super::{IntoShared, SharedRefExt, SharedRefMutExt, UniqueRefExt, UniqueRefMutExt};
}

fn bytes_to_ptr(typed_bytes: TypedBytes<'_>) -> AllocationPointer {
    let bytes = typed_bytes.bytes().bytes().unwrap();
    assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());
    let mut read = Cursor::new(bytes);
    AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
}

pub unsafe trait SharedTrait: TypeTrait {}
pub unsafe trait UniqueTrait: SharedTrait {}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Unique {
    pub child_ty: Box<TypeEnum>,
}

impl Unique {
    pub fn new(child_ty: impl Into<TypeEnum>) -> Self {
        Self { child_ty: Box::new(child_ty.into()) }
    }
}

impl Display for Unique {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Unique<{}>", self.child_ty))
    }
}

impl SizedTypeExt for Unique {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }
}

impl TypeExt for Unique {
    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        dbg!(&self.child_ty);
        dbg!(&other.child_ty);
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    fn has_safe_binary_representation(&self) -> bool {
        false
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
    fn deref(&self) -> RefAny<'_>;
}

pub trait UniqueRefMutExt<'a> {
    fn deref_mut(&mut self) -> RefMutAny<'_>;
}

impl<'a, T> UniqueRefMutExt<'a> for T
where T: RefMutExt<'a, Unique> + 'a
{
    fn deref_mut(&mut self) -> RefMutAny<'_> {
        let typed_bytes = unsafe { self.pointee_typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        unsafe {
            let typed_bytes = Allocator::get().deref_mut_ptr(ptr).unwrap();

            RefMutAny::from(typed_bytes)
        }
    }
}

pub trait IntoShared<'a>: RefMutDynExt<'a> {
    type Target<T: TypeTrait>;

    fn into_shared(self, handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared>;
}

unsafe fn change_type_to_shared<'a>(reference: &(impl RefMutDynExt<'a> + IntoShared<'a>)) {
    let ptr = bytes_to_ptr(reference.pointee_typed_bytes());
    Allocator::get()
        .map_type(ptr, |ty| {
            let unique_ty = ty.downcast_ref::<Unique>().unwrap();
            let child_ty = unique_ty.child_ty.as_ref().clone();
            *ty = Shared::new(child_ty).into();
        })
        .unwrap();
}

impl<'a> IntoShared<'a> for RefMut<'a, Unique> {
    type Target<T: TypeTrait> = RefMut<'a, T>;

    fn into_shared(self, _handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared> {
        unsafe {
            change_type_to_shared(&self);
            RefMut::from(self.into_pointee_typed_bytes()).downcast_mut().unwrap()
        }
    }
}

impl<'a> IntoShared<'a> for OwnedRefMut<'a, Unique> {
    type Target<T: TypeTrait> = OwnedRefMut<'a, T>;

    fn into_shared(self, _handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared> {
        unsafe {
            change_type_to_shared(&self);
            self.into_mut_any().downcast_mut().unwrap()
        }
    }
}

impl<'a, T> UniqueRefExt<'a> for T
where T: RefExt<'a, Unique>
{
    fn deref(&self) -> RefAny<'_> {
        let typed_bytes = unsafe { self.pointee_typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        unsafe {
            let typed_bytes = Allocator::get().deref_ptr(ptr).unwrap();
            RefAny::from(typed_bytes)
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Shared {
    pub child_ty: Box<TypeEnum>,
}

impl Shared {
    pub fn new(child_ty: impl Into<TypeEnum>) -> Self {
        Self { child_ty: Box::new(child_ty.into()) }
    }
}

impl Display for Shared {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Shared<{}>", self.child_ty))
    }
}

impl SizedTypeExt for Shared {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }
}

impl TypeExt for Shared {
    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    fn has_safe_binary_representation(&self) -> bool {
        false
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
    fn deref(&self) -> RefAny<'_>;
}

pub trait SharedRefMutExt<'a> {}

impl<'a, T> SharedRefExt<'a> for T
where T: RefExt<'a, Shared>
{
    fn deref(&self) -> RefAny<'_> {
        let typed_bytes = unsafe { self.pointee_typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        unsafe {
            let typed_bytes = Allocator::get().deref_ptr(ptr).unwrap();
            RefAny::from(typed_bytes)
        }
    }
}

impl<'a, T> SharedRefMutExt<'a> for T where T: RefMutExt<'a, Shared> {}
