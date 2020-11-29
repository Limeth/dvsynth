use super::{
    AllocationPointer, IndirectInnerRef, IndirectInnerRefMut, InnerRef, InnerRefMut, InnerRefTypes, TypeEnum,
    TypeTrait,
};
use crate::graph::alloc::{AllocatedType, Allocator};
use crate::graph::NodeIndex;
use crate::node::behaviour::AllocatorHandle;
use crate::node::ty::DynTypeTrait;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

// #[derive(Clone, Copy)]
// pub(crate) enum PointerType<'a, T: TypeTrait> {
//     Direct { data: &'a [u8], ty: &'a T },
//     Indirect(AllocationPointer),
// }

// impl<'a, T: TypeTrait> PointerType<'a, T> {
//     pub(crate) fn deref(self) -> (&'a dyn AllocatedType, &'a T) {
//         use PointerType::*;
//         match self {
//             Direct { data, ty } => (data, ty),
//             Indirect(ptr) => Allocator::get().deref(),
//         }
//     }
// }

// pub(crate) enum PointerMutType<'a, T: TypeTrait> {
//     Direct { data: &'a mut [u8], ty: &'a T },
//     Indirect(AllocationPointer),
// }

/// A common trait for references that allow for shared access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait RefExt<'a, T: TypeTrait>: RefDynExt<'a> {
    fn get_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a>;
}

/// A common trait for references that allow for mutable access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait RefMutExt<'a, T: TypeTrait>: RefExt<'a, T> + RefMutDynExt<'a> {
    fn get_mut_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRefMut<'a>;
}

pub trait RefDynExt<'a> {
    unsafe fn bytes(&self) -> &[u8];
    fn ty_equals(&self, ty: &TypeEnum) -> bool;
}

pub trait RefMutDynExt<'a>: RefDynExt<'a> {}

macro_rules! impl_ref_dyn_exts {
    (impl[$($generics:tt)*] RefDynExt<'a> for $($rest:tt)*) => {
        impl<$($generics)*> RefDynExt<'a> for $($rest)* {
            unsafe fn bytes(&self) -> &[u8] {
                self.get_inner().raw_bytes()
            }

            fn ty_equals(&self, other_ty: &TypeEnum) -> bool {
                let (_, ty) = unsafe { self.get_inner().deref_ref().unwrap() };

                other_ty.downcast_ref() == Some(ty)
            }
        }
    };

    (impl[$($generics:tt)*] RefMutDynExt<'a> for $($rest:tt)*) => {
        impl<$($generics)*> RefMutDynExt<'a> for $($rest)* {
        }
    };
}

/// A refcounted mutable reference to `T`.
pub struct OwnedRefMut<T>
where T: DynTypeTrait
{
    ptr: AllocationPointer,
    node: NodeIndex,
    __marker: PhantomData<T>,
}

impl<T> OwnedRefMut<T>
where T: DynTypeTrait
{
    pub fn allocate(descriptor: T::Descriptor, handle: AllocatorHandle<'_>) -> Self
    where T: DynTypeTrait {
        Self {
            ptr: Allocator::get().allocate::<T>(descriptor, handle),
            node: handle.node,
            __marker: Default::default(),
        }
    }

    fn from_ref_mut<'a>(reference: RefMut<'a, T>, handle: AllocatorHandle<'a>) -> Self {
        unsafe {
            Allocator::get()
                .refcount_owned_increment(reference.inner.ptr, handle.node)
                .expect("Could not increment the refcount of a RefMut while converting to OwnedRefMut.");
        }

        Self { ptr: reference.inner.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn to_owned_ref(self, handle: AllocatorHandle<'_>) -> OwnedRef<T> {
        OwnedRef { ptr: self.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn to_mut<'a>(self, _handle: AllocatorHandle<'a>) -> RefMut<'a, T> {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while converting to RefMut.");
        }

        let inner = IndirectInnerRefMut::new(self.ptr);

        RefMut { inner, __marker: Default::default() }
    }

    pub fn to_ref<'a>(self, _handle: AllocatorHandle<'a>) -> Ref<'a, T> {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while converting to Ref.");
        }

        let inner = IndirectInnerRef::new(self.ptr);

        Ref { inner, __marker: Default::default() }
    }
}

impl<'a, T> RefExt<'a, T> for &'a OwnedRefMut<T>
where T: DynTypeTrait
{
    fn get_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a> {
        IndirectInnerRef::new(self.ptr)
    }
}

impl<'a, T> RefExt<'a, T> for &'a mut OwnedRefMut<T>
where T: DynTypeTrait
{
    fn get_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a> {
        IndirectInnerRef::new(self.ptr)
    }
}

impl<'a, T> RefMutExt<'a, T> for &'a mut OwnedRefMut<T>
where T: DynTypeTrait
{
    fn get_mut_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRefMut<'a> {
        IndirectInnerRefMut::new(self.ptr)
    }
}

impl_ref_dyn_exts!(impl['a, T: DynTypeTrait] RefDynExt<'a> for &'a OwnedRefMut<T>);
impl_ref_dyn_exts!(impl['a, T: DynTypeTrait] RefDynExt<'a> for &'a mut OwnedRefMut<T>);
impl_ref_dyn_exts!(impl['a, T: DynTypeTrait] RefMutDynExt<'a> for &'a mut OwnedRefMut<T>);

impl<T> Drop for OwnedRefMut<T>
where T: DynTypeTrait
{
    fn drop(&mut self) {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while dropping.");
        }
    }
}

/// A refcounted shared reference to `T`.
#[derive(Clone)]
pub struct OwnedRef<T>
where T: DynTypeTrait
{
    ptr: AllocationPointer,
    node: NodeIndex,
    __marker: PhantomData<T>,
}

impl<T> OwnedRef<T>
where T: DynTypeTrait
{
    fn from_ref<'a>(reference: Ref<'a, T>, handle: AllocatorHandle<'a>) -> Self {
        unsafe {
            Allocator::get()
                .refcount_owned_increment(reference.inner.ptr, handle.node)
                .expect("Could not increment the refcount of a Ref while converting to OwnedRef.");
        }

        Self { ptr: reference.inner.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn to_ref<'a>(self, _handle: AllocatorHandle<'a>) -> Ref<'a, T> {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while converting to Ref.");
        }

        let inner = IndirectInnerRef::new(self.ptr);

        Ref { inner, __marker: Default::default() }
    }
}

impl<'a, T> RefExt<'a, T> for &'a OwnedRef<T>
where T: DynTypeTrait
{
    fn get_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a> {
        IndirectInnerRef::new(self.ptr)
    }
}

impl_ref_dyn_exts!(impl['a, T: DynTypeTrait] RefDynExt<'a> for &'a OwnedRef<T>);

impl<T> Drop for OwnedRef<T>
where T: DynTypeTrait
{
    fn drop(&mut self) {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while dropping.");
        }
    }
}

/// A non-refcounted mutable reference to `T`.
#[repr(transparent)]
pub struct RefMut<'a, T>
where T: TypeTrait
{
    inner: <T::InnerRefTypes as InnerRefTypes<T>>::InnerRefMut<'a>,
    // ptr: AllocationPointer,
    __marker: PhantomData<(&'a mut T, *mut T)>,
}

impl<'a, T> RefMut<'a, T>
where T: TypeTrait
{
    pub fn to_ref(self, _handle: AllocatorHandle<'a>) -> Ref<'a, T> {
        Ref { inner: T::InnerRefTypes::downgrade(self.inner), __marker: Default::default() }
    }
}

impl<'a, T> RefMut<'a, T>
where T: DynTypeTrait
{
    pub fn to_owned_mut(self, handle: AllocatorHandle<'a>) -> OwnedRefMut<T> {
        OwnedRefMut::from_ref_mut(self, handle)
    }

    pub fn to_owned_ref(self, handle: AllocatorHandle<'a>) -> OwnedRef<T> {
        OwnedRef::from_ref(self.to_ref(handle), handle)
    }
}

impl<'a, T> RefExt<'a, T> for RefMut<'a, T>
where T: TypeTrait
{
    fn get_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a> {
        T::InnerRefTypes::downgrade(self.inner)
    }
}

impl<'a, T> RefMutExt<'a, T> for RefMut<'a, T>
where T: TypeTrait
{
    fn get_mut_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRefMut<'a> {
        self.inner
    }
}

impl_ref_dyn_exts!(impl['a, T: TypeTrait] RefDynExt<'a> for RefMut<'a, T>);
impl_ref_dyn_exts!(impl['a, T: TypeTrait] RefMutDynExt<'a> for RefMut<'a, T>);

/// A non-refcounted shared reference to `T`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Ref<'a, T>
where T: TypeTrait
{
    inner: <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a>,
    // ptr: AllocationPointer,
    __marker: PhantomData<(&'a T, *const T)>,
}

impl<'a, T> Ref<'a, T>
where T: DynTypeTrait
{
    pub fn to_owned_ref(self, handle: AllocatorHandle<'a>) -> OwnedRef<T> {
        OwnedRef::from_ref(self, handle)
    }
}

impl<'a, T> RefExt<'a, T> for Ref<'a, T>
where T: TypeTrait
{
    fn get_inner(self) -> <T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a> {
        self.inner
    }
}

impl_ref_dyn_exts!(impl['a, T: TypeTrait] RefDynExt<'a> for Ref<'a, T>);

pub struct RefMutAny<'a> {
    bytes: &'a mut [u8],
    ty: &'a TypeEnum,
}

impl<'a> RefMutAny<'a> {
    pub unsafe fn from(bytes: &'a mut [u8], ty: &'a TypeEnum) -> Self {
        Self { bytes, ty }
    }

    pub fn downcast_mut<T: TypeTrait>(self, handle: AllocatorHandle<'a>) -> Option<RefMut<'a, T>> {
        self.ty
            .downcast_ref::<T>()
            .and_then(move |ty: &'a T| unsafe {
                <<T::InnerRefTypes as InnerRefTypes<T>>::InnerRefMut<'a> as InnerRefMut<'a>>::from_raw_bytes(
                    self.bytes, ty, handle,
                )
                .ok()
            })
            .map(|inner| RefMut { inner, __marker: Default::default() })
    }
}

impl<'a> RefDynExt<'a> for RefMutAny<'a> {
    unsafe fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn ty_equals(&self, ty: &TypeEnum) -> bool {
        ty == self.ty
    }
}

impl<'a> RefMutDynExt<'a> for RefMutAny<'a> {}

#[derive(Clone, Copy)]
pub struct RefAny<'a> {
    bytes: &'a [u8],
    ty: &'a TypeEnum,
}

impl<'a> RefAny<'a> {
    pub unsafe fn from(bytes: &'a [u8], ty: &'a TypeEnum) -> Self {
        Self { bytes, ty }
    }

    pub fn downcast_ref<T: TypeTrait>(self, handle: AllocatorHandle<'a>) -> Option<Ref<'a, T>> {
        self.ty
            .downcast_ref::<T>()
            .and_then(move |ty: &'a T| unsafe {
                <<T::InnerRefTypes as InnerRefTypes<T>>::InnerRef<'a> as InnerRef<'a>>::from_raw_bytes(
                    self.bytes, ty, handle,
                )
                .ok()
            })
            .map(|inner| Ref { inner, __marker: Default::default() })
    }
}

impl<'a> RefDynExt<'a> for RefAny<'a> {
    unsafe fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn ty_equals(&self, ty: &TypeEnum) -> bool {
        ty == self.ty
    }
}
