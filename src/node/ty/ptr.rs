use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt::Display;
use std::io::Cursor;

use crate::graph::alloc::Allocator;
use crate::node::behaviour::AllocatorHandle;

use super::{
    AllocationPointer, BorrowedRefAny, BorrowedRefMut, BorrowedRefMutAny, Bytes, CloneableTypeExt,
    DowncastFromTypeEnum, OwnedRefMut, Ref, RefAnyExt, RefMut, RefMutAny, SizedTypeExt, TypeEnum, TypeExt,
    TypeTrait, TypedBytes,
};

pub mod prelude {
    pub use super::{IntoShared, SharedRefExt, SharedRefMutExt, UniqueRefExt, UniqueRefMutExt};
}

pub fn is_pointer(ty: &TypeEnum) -> bool {
    ty.downcast_ref::<Shared>().is_some() || ty.downcast_ref::<Unique>().is_some()
}

pub fn bytes_to_ptr(bytes: Bytes<'_>) -> AllocationPointer {
    let bytes = bytes.bytes().unwrap();
    assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());
    let mut read = Cursor::new(bytes);
    AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
}

pub fn typed_bytes_to_ptr(typed_bytes: TypedBytes<'_>) -> Option<AllocationPointer> {
    if is_pointer(typed_bytes.borrow().ty().as_ref()) {
        let bytes = typed_bytes.bytes().bytes().unwrap();
        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());
        let mut read = Cursor::new(bytes);
        Some(AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap()))
    } else {
        None
    }
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

unsafe impl SizedTypeExt for Unique {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }
}

unsafe impl TypeExt for Unique {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: Bytes<'a>) -> Vec<TypedBytes<'a>> {
        let ptr = bytes_to_ptr(data.borrow());
        let typed_bytes = Allocator::get().deref_ptr(ptr).unwrap();
        vec![typed_bytes]
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }
}

impl From<Unique> for TypeEnum {
    fn from(other: Unique) -> Self {
        TypeEnum::Unique(other)
    }
}

impl_downcast_from_type_enum!(Unique(Unique));

impl TypeTrait for Unique {}
unsafe impl SharedTrait for Unique {}
unsafe impl UniqueTrait for Unique {}

pub trait UniqueRefExt<'a> {
    fn deref(&self) -> BorrowedRefAny<'_>;
}

pub trait UniqueRefMutExt<'a> {
    fn deref_mut(&mut self) -> BorrowedRefMutAny<'_>;
}

impl<'a, T> UniqueRefMutExt<'a> for T
where T: RefMut<'a, Unique> + 'a
{
    fn deref_mut(&mut self) -> BorrowedRefMutAny<'_> {
        let (rc, typed_bytes) = unsafe { self.rc_and_typed_bytes_mut() };
        let ptr = typed_bytes_to_ptr(typed_bytes.borrow()).unwrap();

        unsafe {
            let typed_bytes = Allocator::get().deref_mut_ptr(ptr).unwrap();
            BorrowedRefMutAny::from(typed_bytes, rc)
        }
    }
}

pub trait IntoShared<'a>: RefMutAny<'a> {
    type Target<T: TypeTrait>;

    fn into_shared(self, handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared>;
}

unsafe fn change_type_to_shared<'a>(reference: &(impl RefMutAny<'a> + IntoShared<'a>)) {
    let ptr = typed_bytes_to_ptr(reference.typed_bytes()).unwrap();
    Allocator::get()
        .map_type(ptr, |ty| {
            let unique_ty = ty.downcast_ref::<Unique>().unwrap();
            let child_ty = unique_ty.child_ty.as_ref().clone();
            *ty = Shared::new(child_ty).into();
        })
        .unwrap();
}

impl<'a> IntoShared<'a> for BorrowedRefMut<'a, Unique> {
    type Target<T: TypeTrait> = BorrowedRefMut<'a, T>;

    fn into_shared(self, _handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared> {
        unsafe {
            change_type_to_shared(&self);
            BorrowedRefMut::from(self.typed_bytes, self.rc).downcast_mut().unwrap()
        }
    }
}

impl<'a> IntoShared<'a> for OwnedRefMut<'a, Unique> {
    type Target<T: TypeTrait> = OwnedRefMut<'a, T>;

    fn into_shared(self, _handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared> {
        unsafe {
            change_type_to_shared(&self);
            self.into_shared()
        }
    }
}

impl<'a, T> UniqueRefExt<'a> for T
where T: Ref<'a, Unique>
{
    fn deref(&self) -> BorrowedRefAny<'_> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ptr = typed_bytes_to_ptr(typed_bytes).unwrap();

        unsafe {
            let typed_bytes = Allocator::get().deref_ptr(ptr).unwrap();
            BorrowedRefAny::from(typed_bytes, self.refcounter())
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

unsafe impl SizedTypeExt for Shared {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }
}

unsafe impl CloneableTypeExt for Shared {}

unsafe impl TypeExt for Shared {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: Bytes<'a>) -> Vec<TypedBytes<'a>> {
        let ptr = bytes_to_ptr(data.borrow());
        let typed_bytes = Allocator::get().deref_ptr(ptr).unwrap();
        vec![typed_bytes]
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn is_cloneable(&self) -> bool {
        true
    }
}

impl From<Shared> for TypeEnum {
    fn from(other: Shared) -> Self {
        TypeEnum::Shared(other)
    }
}

impl_downcast_from_type_enum!(Shared(Shared));

impl TypeTrait for Shared {}
unsafe impl SharedTrait for Shared {}

pub trait SharedRefExt<'a> {
    fn deref(&self) -> BorrowedRefAny<'_>;
}

pub trait SharedRefMutExt<'a> {}

impl<'a, T> SharedRefExt<'a> for T
where T: Ref<'a, Shared>
{
    fn deref(&self) -> BorrowedRefAny<'_> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ptr = typed_bytes_to_ptr(typed_bytes).unwrap();

        unsafe {
            let typed_bytes = Allocator::get().deref_ptr(ptr).unwrap();
            BorrowedRefAny::from(typed_bytes, self.refcounter())
        }
    }
}

impl<'a, T> SharedRefMutExt<'a> for T where T: RefMut<'a, Shared> {}
