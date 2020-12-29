use byteorder::{LittleEndian, ReadBytesExt};
use std::fmt::Display;
use std::io::Cursor;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::graph::alloc::Allocator;
use crate::node::behaviour::AllocatorHandle;

use super::{
    AllocationPointer, BorrowedRef, BorrowedRefMut, Bytes, CloneableTypeExt, DowncastFromTypeEnum,
    OwnedRefMut, Ref, RefAnyExt, RefMut, RefMutAny, SizedTypeExt, TypeDesc, TypeEnum, TypeExt,
    TypeResolution, TypeTrait, TypedBytes,
};

pub mod prelude {
    pub use super::{IntoShared, SharedRefExt, SharedRefMutExt, UniqueRefExt, UniqueRefMutExt};
}

pub fn is_pointer(ty: &TypeEnum) -> bool {
    ty.resolve_ref::<Shared>().is_some() || ty.resolve_ref::<Unique>().is_some()
}

pub fn bytes_to_ptr(bytes: Bytes<'_>) -> AllocationPointer {
    let bytes = bytes.bytes().unwrap();
    assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());
    let mut read = Cursor::new(bytes);
    AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
}

pub fn typed_bytes_to_ptr(typed_bytes: TypedBytes<'_>) -> Option<AllocationPointer> {
    // dbg!(&typed_bytes);
    // dbg!(backtrace::Backtrace::new());
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
pub struct Unique<T: TypeDesc = !> {
    pub child_ty: Box<TypeEnum>,
    __marker: PhantomData<T>,
}

impl Unique<!> {
    pub fn from_enum(child_ty: impl Into<TypeEnum>) -> Self {
        Self { child_ty: Box::new(child_ty.into()), __marker: Default::default() }
    }

    pub fn downcast_child<T: TypeDesc>(self) -> Option<Unique<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            Some(Unique { child_ty: self.child_ty, __marker: Default::default() })
        } else {
            None
        }
    }

    pub fn downcast_child_ref<T: TypeDesc>(&self) -> Option<&Unique<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute::<&Self, &Unique<T>>(self) })
        } else {
            None
        }
    }

    pub fn downcast_child_mut<T: TypeDesc>(&mut self) -> Option<&mut Unique<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute::<&mut Self, &mut Unique<T>>(self) })
        } else {
            None
        }
    }
}

impl<T: TypeTrait> Unique<T> {
    pub fn new(child_ty: T) -> Self {
        Self { child_ty: Box::new(child_ty.into()), __marker: Default::default() }
    }
}

impl<T: TypeDesc> Unique<T> {
    pub fn upcast(self) -> Unique<!> {
        Unique { child_ty: self.child_ty, __marker: Default::default() }
    }
}

impl<T: TypeDesc> Display for Unique<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Unique<{}>", self.child_ty))
    }
}

unsafe impl<T: TypeDesc> SizedTypeExt for Unique<T> {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }
}

unsafe impl<T: TypeDesc> TypeExt for Unique<T> {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        let ptr = typed_bytes_to_ptr(data.borrow()).unwrap();
        let typed_bytes = Allocator::get().deref_ptr(ptr, data.refcounter()).unwrap();
        vec![typed_bytes]
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }
}

impl<T: TypeDesc> From<Unique<T>> for TypeEnum {
    fn from(other: Unique<T>) -> Self {
        TypeEnum::Unique(other.upcast())
    }
}

impl<T: TypeDesc> DowncastFromTypeEnum for Unique<T> {
    fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
    where Self: Sized {
        if let TypeEnum::Unique(inner) = from {
            inner.downcast_child::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>> {
        if let TypeEnum::Unique(inner) = from {
            inner.downcast_child_ref::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>> {
        if let TypeEnum::Unique(inner) = from {
            inner.downcast_child_mut::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }
}

unsafe impl<T: TypeDesc> TypeDesc for Unique<T> {}
impl<T: TypeDesc> TypeTrait for Unique<T> {}
unsafe impl<T: TypeDesc> SharedTrait for Unique<T> {}
unsafe impl<T: TypeDesc> UniqueTrait for Unique<T> {}

pub trait UniqueRefExt<'a, C: TypeDesc> {
    fn deref(&self) -> BorrowedRef<'_, C>;
}

pub trait UniqueRefMutExt<'a, C: TypeDesc> {
    fn deref_mut(&mut self) -> BorrowedRefMut<'_, C>;
}

impl<'a, T, C> UniqueRefMutExt<'a, C> for T
where
    T: RefMut<'a, Unique<C>> + 'a,
    C: TypeDesc,
{
    fn deref_mut(&mut self) -> BorrowedRefMut<'_, C> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let ptr = typed_bytes_to_ptr(typed_bytes.borrow()).unwrap();

        unsafe {
            let typed_bytes = Allocator::get().deref_mut_ptr(ptr, typed_bytes.refcounter_mut()).unwrap();
            BorrowedRefMut::from_unchecked_type(typed_bytes)
        }
    }
}

pub trait IntoShared<'a>: RefMutAny<'a> {
    type Target<T: TypeTrait>;

    fn into_shared(self, handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared>;
}

// TODO
// unsafe fn change_type_to_shared<'a>(reference: &(impl RefMutAny<'a> + IntoShared<'a>)) {
//     let ptr = typed_bytes_to_ptr(reference.typed_bytes()).unwrap();
//     Allocator::get()
//         .map_type(ptr, |ty| {
//             let unique_ty = ty.downcast_ref::<Unique>().unwrap();
//             let child_ty = unique_ty.child_ty.as_ref().clone();
//             *ty = Shared::new(child_ty).into();
//         })
//         .unwrap();
// }

// impl<'a> IntoShared<'a> for BorrowedRefMut<'a, Unique> {
//     type Target<T: TypeTrait> = BorrowedRefMut<'a, T>;

//     fn into_shared(self, _handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared> {
//         unsafe {
//             change_type_to_shared(&self);
//             BorrowedRefMut::from(self.typed_bytes, self.rc).downcast_mut().unwrap()
//         }
//     }
// }

// impl<'a> IntoShared<'a> for OwnedRefMut<'a, Unique> {
//     type Target<T: TypeTrait> = OwnedRefMut<'a, T>;

//     fn into_shared(self, _handle: AllocatorHandle<'a, '_>) -> Self::Target<Shared> {
//         unsafe {
//             change_type_to_shared(&self);
//             self.into_shared()
//         }
//     }
// }

impl<'a, T, C> UniqueRefExt<'a, C> for T
where
    T: Ref<'a, Unique<C>>,
    C: TypeDesc,
{
    fn deref(&self) -> BorrowedRef<'_, C> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ptr = typed_bytes_to_ptr(typed_bytes.borrow()).unwrap();

        unsafe {
            let typed_bytes = Allocator::get().deref_ptr(ptr, typed_bytes.refcounter()).unwrap();
            BorrowedRef::from_unchecked_type(typed_bytes)
        }
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct Shared<T: TypeDesc = !> {
    pub child_ty: Box<TypeEnum>,
    __marker: PhantomData<T>,
}

impl Shared<!> {
    pub fn from_enum(child_ty: impl Into<TypeEnum>) -> Self {
        Self { child_ty: Box::new(child_ty.into()), __marker: Default::default() }
    }

    pub fn downcast_child<T: TypeDesc>(self) -> Option<Shared<T>> {
        if self.child_ty.downcast_ref::<T>().is_some() {
            Some(Shared { child_ty: self.child_ty, __marker: Default::default() })
        } else {
            None
        }
    }

    pub fn downcast_child_ref<T: TypeDesc>(&self) -> Option<&Shared<T>> {
        if self.child_ty.downcast_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute(self) })
        } else {
            None
        }
    }

    pub fn downcast_child_mut<T: TypeDesc>(&mut self) -> Option<&mut Shared<T>> {
        if self.child_ty.downcast_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute(self) })
        } else {
            None
        }
    }
}

impl<T: TypeTrait> Shared<T> {
    pub fn new(child_ty: T) -> Self {
        Self { child_ty: Box::new(child_ty.into()), __marker: Default::default() }
    }
}

impl<T: TypeDesc> Shared<T> {
    pub fn upcast(self) -> Shared<!> {
        Shared { child_ty: self.child_ty, __marker: Default::default() }
    }
}

impl<T: TypeDesc> Display for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Shared<{}>", self.child_ty))
    }
}

unsafe impl<T: TypeDesc> SizedTypeExt for Shared<T> {
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }
}

/// A shared pointer is cloneable even if its contents are not.
unsafe impl<T: TypeDesc> CloneableTypeExt for Shared<T> {}

unsafe impl<T: TypeDesc> TypeExt for Shared<T> {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        let ptr = typed_bytes_to_ptr(data.borrow()).unwrap();
        let typed_bytes = Allocator::get().deref_ptr(ptr, data.refcounter()).unwrap();
        vec![typed_bytes]
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn is_cloneable(&self) -> bool {
        true
    }
}

impl<T: TypeDesc> From<Shared<T>> for TypeEnum {
    fn from(other: Shared<T>) -> Self {
        TypeEnum::Shared(other.upcast())
    }
}

impl<T: TypeDesc> DowncastFromTypeEnum for Shared<T> {
    fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
    where Self: Sized {
        if let TypeEnum::Shared(inner) = from {
            inner.downcast_child::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>> {
        if let TypeEnum::Shared(inner) = from {
            inner.downcast_child_ref::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>> {
        if let TypeEnum::Shared(inner) = from {
            inner.downcast_child_mut::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }
}

unsafe impl<T: TypeDesc> TypeDesc for Shared<T> {}
impl<T: TypeDesc> TypeTrait for Shared<T> {}
unsafe impl<T: TypeDesc> SharedTrait for Shared<T> {}

pub trait SharedRefExt<'a, C: TypeDesc> {
    fn deref(&self) -> BorrowedRef<'_, C>;
}

pub trait SharedRefMutExt<'a, C: TypeDesc> {}

impl<'a, T, C> SharedRefExt<'a, C> for T
where
    T: Ref<'a, Shared<C>>,
    C: TypeDesc,
{
    fn deref(&self) -> BorrowedRef<'_, C> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ptr = typed_bytes_to_ptr(typed_bytes.borrow()).unwrap();

        unsafe {
            let typed_bytes = Allocator::get().deref_ptr(ptr, typed_bytes.refcounter()).unwrap();
            BorrowedRef::from_unchecked_type(typed_bytes)
        }
    }
}

impl<'a, T, C> SharedRefMutExt<'a, C> for T
where
    T: RefMut<'a, Shared<C>>,
    C: TypeDesc,
{
}
