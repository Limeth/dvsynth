use super::{
    AllocationPointer, Shared, SharedTrait, SizedTypeExt, TypeEnum, TypeTrait, TypedBytes, TypedBytesMut,
    Unique, UniqueTrait,
};
use crate::graph::alloc::{AllocatedType, AllocationInner, Allocator};
use crate::graph::NodeIndex;
use crate::node::behaviour::AllocatorHandle;
use crate::node::ty::DynTypeTrait;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::borrow::Cow;
use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub mod prelude {
    pub use super::{RefDynExt, RefExt, RefMutDynExt, RefMutExt};
}

pub unsafe fn visit_recursively<'a>(typed_bytes: TypedBytes<'a>, visit: &mut dyn FnMut(TypedBytes<'_>)) {
    (visit)(typed_bytes.borrow());

    for child in typed_bytes.children() {
        visit_recursively(child, visit);
    }
}

pub unsafe fn refcount_owned_increment_recursive<'a>(reference: &impl RefDynExt<'a>, node: NodeIndex) {
    visit_recursively(reference.pointing_typed_bytes(), &mut |typed_bytes| {
        let ptr = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes).unwrap();
        Allocator::get().refcount_owned_increment(ptr, node).unwrap()
    });
}

pub unsafe fn refcount_owned_decrement_recursive<'a>(reference: &impl RefDynExt<'a>, node: NodeIndex) {
    visit_recursively(reference.pointing_typed_bytes(), &mut |typed_bytes| {
        let ptr = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes).unwrap();
        Allocator::get().refcount_owned_decrement(ptr, node).unwrap()
    });
}

/// A common trait for references that allow for shared access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait RefExt<'a, T: TypeTrait>: RefDynExt<'a> {
    type Ref<'b, R: TypeTrait>: RefExt<'b, R>;
}

/// A common trait for references that allow for mutable access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait RefMutExt<'a, T: TypeTrait>: RefExt<'a, T> + RefMutDynExt<'a> {
    type RefMut<'b, R: TypeTrait>: RefMutExt<'b, R>;
}

pub trait RefDynExt<'a>: Sized {
    /// Data accessed by dereferencing the pointer.
    ///
    /// `Ref`/`RefMut`: The referenced data.
    /// `OwnedRef`/`OwnedRefMut`: The referenced data.
    unsafe fn pointee_typed_bytes<'b>(&'b self) -> TypedBytes<'b>;

    /// Data accessed by reading the pointer itself.
    ///
    /// `Ref`/`RefMut`: The referenced data.
    /// `OwnedRef`/`OwnedRefMut`: The pointer data itself.
    unsafe fn pointing_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.pointee_typed_bytes()
    }
}

pub trait RefMutDynExt<'a>: RefDynExt<'a> {
    unsafe fn pointee_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b>;

    unsafe fn pointing_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        self.pointee_typed_bytes_mut()
    }

    unsafe fn into_pointee_typed_bytes(self) -> TypedBytesMut<'a>;

    // unsafe fn children_mut<'b>(&'b mut self) -> Self::IterMutChildren<'b>;
}

// TODO: Consider allowing the lifetime to be a sub-lifetime of 'state?
// FIXME: Alter refcount recursively
/// A refcounted mutable reference to `T`.
pub struct OwnedRefMut<'state, T> {
    ptr: AllocationPointer,
    node: NodeIndex,
    __marker: PhantomData<&'state T>,
}

impl<'state, T> OwnedRefMut<'state, T> {
    pub(crate) fn into_mut_any(self) -> OwnedRefMutAny<'state> {
        OwnedRefMut { ptr: self.ptr, node: self.node, __marker: Default::default() }
    }
}

impl<'state> OwnedRefMut<'state, ()> {
    fn from_ref_mut<'reference, 'invocation, P: UniqueTrait>(
        reference: RefMut<'reference, P>,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> Self
    where
        'invocation: 'reference,
        'state: 'invocation,
    {
        let typed_bytes = unsafe { reference.pointee_typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        Self { ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn downcast_mut<'invocation, T: TypeTrait>(self) -> Option<OwnedRefMut<'state, T>>
    where 'state: 'invocation {
        let typed_bytes = unsafe { Allocator::get().deref_mut_ptr(self.ptr) }.unwrap();

        typed_bytes.ty().downcast_ref::<T>().map(|_| OwnedRefMut {
            ptr: self.ptr,
            node: self.node,
            __marker: Default::default(),
        })
    }
}

impl<'state, T> OwnedRefMut<'state, T>
where T: DynTypeTrait
{
    pub fn allocate_object<'invocation>(
        descriptor: T::Descriptor,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> Self
    where
        'state: 'invocation,
    {
        Self {
            ptr: Allocator::get().allocate_object::<T>(descriptor, handle),
            node: handle.node,
            __marker: Default::default(),
        }
    }
}

impl<'state, T> OwnedRefMut<'state, T>
where T: TypeTrait + SizedTypeExt
{
    pub fn allocate_bytes<'invocation>(ty: T, handle: AllocatorHandle<'invocation, 'state>) -> Self
    where 'state: 'invocation {
        Self {
            ptr: Allocator::get().allocate_bytes::<T>(ty, handle),
            node: handle.node,
            __marker: Default::default(),
        }
    }
}

impl<'state, T> OwnedRefMut<'state, T>
where T: TypeTrait
{
    pub fn to_owned_ref<'invocation>(
        self,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> OwnedRef<'state, T>
    where
        'state: 'invocation,
    {
        OwnedRef { ptr: self.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn to_mut<'invocation>(
        self,
        _handle: AllocatorHandle<'invocation, 'state>,
    ) -> RefMut<'invocation, T>
    where
        'state: 'invocation,
    {
        let typed_bytes = unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while converting to Ref.");

            Allocator::get().deref_mut_ptr(self.ptr).unwrap()
        };

        RefMut { typed_bytes, __marker: Default::default() }
    }

    pub fn to_ref<'invocation>(self, _handle: AllocatorHandle<'invocation, 'state>) -> Ref<'invocation, T> {
        let typed_bytes = unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while converting to Ref.");

            Allocator::get().deref_ptr(self.ptr).unwrap()
        };

        Ref { typed_bytes, __marker: Default::default() }
    }
}

impl<'a, T> RefExt<'a, T> for OwnedRefMut<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = OwnedRef<'b, R>;
}

impl<'a, T> RefMutExt<'a, T> for OwnedRefMut<'a, T>
where T: TypeTrait
{
    type RefMut<'b, R: TypeTrait> = OwnedRefMut<'b, R>;
}

impl<'a, T> RefDynExt<'a> for OwnedRefMut<'a, T> {
    unsafe fn pointee_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        Allocator::get().deref_ptr(self.ptr).unwrap()
    }

    unsafe fn pointing_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        let typed_bytes = Allocator::get().deref_ptr(self.ptr).unwrap();
        let ptr_ty = Unique::new(typed_bytes.borrow().ty().into_owned()).into();

        TypedBytes::from(self.ptr.as_bytes(), Cow::Owned(ptr_ty))
    }
}

impl<'a, T> RefMutDynExt<'a> for OwnedRefMut<'a, T> {
    unsafe fn pointee_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        Allocator::get().deref_mut_ptr(self.ptr).unwrap()
    }

    unsafe fn pointing_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        let typed_bytes = Allocator::get().deref_mut_ptr(self.ptr).unwrap();
        let ptr_ty = Unique::new(typed_bytes.borrow().ty().into_owned()).into();

        TypedBytesMut::from(self.ptr.as_bytes_mut(), Cow::Owned(ptr_ty))
    }

    unsafe fn into_pointee_typed_bytes(self) -> TypedBytesMut<'a> {
        Allocator::get().deref_mut_ptr(self.ptr).unwrap()
    }
}

impl<'a, T> Drop for OwnedRefMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while dropping.");
        }
    }
}

// TODO: Consider allowing the lifetime to be a sub-lifetime of 'state?
// FIXME: Alter refcount recursively
/// A refcounted shared reference to `T`.
#[derive(Clone)]
pub struct OwnedRef<'state, T> {
    ptr: AllocationPointer,
    node: NodeIndex,
    __marker: PhantomData<&'state T>,
}

impl<'state> OwnedRef<'state, ()> {
    fn from_ref<'reference, 'invocation, P: SharedTrait>(
        reference: Ref<'reference, P>,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> Self
    where
        'invocation: 'reference,
        'state: 'invocation,
    {
        let typed_bytes = unsafe { reference.pointee_typed_bytes() };
        let bytes = typed_bytes.bytes().bytes().unwrap();

        assert_eq!(bytes.len(), std::mem::size_of::<AllocationPointer>());

        let ptr = {
            let mut read = Cursor::new(bytes);
            AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap())
        };

        Self { ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn downcast_ref<'invocation, T: TypeTrait>(self) -> Option<OwnedRef<'state, T>>
    where 'state: 'invocation {
        let typed_bytes = unsafe { Allocator::get().deref_ptr(self.ptr) }.unwrap();

        typed_bytes.ty().downcast_ref::<T>().map(|_| OwnedRef {
            ptr: self.ptr,
            node: self.node,
            __marker: Default::default(),
        })
    }
}

impl<'state, T> OwnedRef<'state, T>
where T: TypeTrait
{
    pub fn to_ref<'invocation>(self, _handle: AllocatorHandle<'invocation, 'state>) -> Ref<'invocation, T>
    where 'state: 'invocation {
        let typed_bytes = unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while converting to Ref.");

            Allocator::get().deref_ptr(self.ptr).unwrap()
        };

        Ref { typed_bytes, __marker: Default::default() }
    }
}

impl<'state, T> OwnedRef<'state, T> {
    unsafe fn shared_ref_bytes(&self) -> TypedBytes<'_> {
        let typed_bytes = Allocator::get().deref_ptr(self.ptr).unwrap();
        let ptr_ty = Unique::new(typed_bytes.borrow().ty().into_owned()).into();

        TypedBytes::from(self.ptr.as_bytes(), Cow::Owned(ptr_ty))
    }
}

impl<'a, T> RefExt<'a, T> for OwnedRef<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = OwnedRef<'b, R>;
}

impl<'a, T> RefDynExt<'a> for OwnedRef<'a, T> {
    unsafe fn pointee_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        Allocator::get().deref_ptr(self.ptr).unwrap()
    }

    unsafe fn pointing_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.shared_ref_bytes()
    }
}

impl<'a, T> Drop for OwnedRef<'a, T> {
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
pub struct RefMut<'a, T> {
    typed_bytes: TypedBytesMut<'a>,
    __marker: PhantomData<(&'a mut T, *mut T)>,
}

impl<'a> RefMut<'a, ()> {
    pub unsafe fn from(typed_bytes: TypedBytesMut<'a>) -> Self {
        Self { typed_bytes, __marker: Default::default() }
    }

    pub fn downcast_mut<'state: 'a, T: TypeTrait>(self) -> Option<RefMut<'a, T>> {
        if self.typed_bytes.borrow().ty().downcast_ref::<T>().is_some() {
            Some(RefMut { typed_bytes: self.typed_bytes, __marker: Default::default() })
        } else {
            None
        }
    }
}

impl<'a, T> RefMut<'a, T>
where T: TypeTrait
{
    pub fn to_ref<'state: 'a>(self, _handle: AllocatorHandle<'a, 'state>) -> Ref<'a, T> {
        Ref { typed_bytes: self.typed_bytes.downgrade(), __marker: Default::default() }
    }
}

impl<'a, P: UniqueTrait> RefMut<'a, P> {
    pub fn to_owned_mut<'invocation, 'state>(
        self,
        handle: AllocatorHandle<'a, 'state>,
    ) -> OwnedRefMut<'state, ()>
    where
        'invocation: 'a,
        'state: 'invocation,
    {
        OwnedRefMut::from_ref_mut(self, handle)
    }
}

impl<'a, P: SharedTrait> RefMut<'a, P> {
    pub fn to_owned_ref<'invocation, 'state>(
        self,
        handle: AllocatorHandle<'a, 'state>,
    ) -> OwnedRef<'state, ()>
    where
        'invocation: 'a,
        'state: 'invocation,
    {
        OwnedRef::from_ref(self.to_ref(handle), handle)
    }
}

impl<'a, T> RefExt<'a, T> for RefMut<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = Ref<'b, R>;
}

impl<'a, T> RefMutExt<'a, T> for RefMut<'a, T>
where T: TypeTrait
{
    type RefMut<'b, R: TypeTrait> = RefMut<'b, R>;
}

impl<'a, T> RefDynExt<'a> for RefMut<'a, T> {
    unsafe fn pointee_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.typed_bytes.borrow()
    }
}

impl<'a, T> RefMutDynExt<'a> for RefMut<'a, T> {
    unsafe fn pointee_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        self.typed_bytes.borrow_mut()
    }

    unsafe fn into_pointee_typed_bytes(self) -> TypedBytesMut<'a> {
        self.typed_bytes
    }
}

/// A non-refcounted shared reference to `T`.
#[derive(Clone)]
#[repr(transparent)]
pub struct Ref<'a, T> {
    typed_bytes: TypedBytes<'a>,
    __marker: PhantomData<(&'a T, *const T)>,
}

impl<'a> Ref<'a, ()> {
    pub unsafe fn from(typed_bytes: TypedBytes<'a>) -> Self {
        Self { typed_bytes, __marker: Default::default() }
    }

    pub fn downcast_ref<'state: 'a, T: TypeTrait>(self) -> Option<Ref<'a, T>> {
        if self.typed_bytes.borrow().ty().downcast_ref::<T>().is_some() {
            Some(Ref { typed_bytes: self.typed_bytes, __marker: Default::default() })
        } else {
            None
        }
    }
}

impl<'a, P: SharedTrait> Ref<'a, P> {
    pub fn to_owned_ref<'invocation, 'state>(
        self,
        handle: AllocatorHandle<'a, 'state>,
    ) -> OwnedRef<'state, ()>
    where
        'invocation: 'a,
        'state: 'invocation,
    {
        OwnedRef::from_ref(self, handle)
    }
}

impl<'a, T> RefExt<'a, T> for Ref<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = Ref<'b, R>;
}

impl<'a, T> RefDynExt<'a> for Ref<'a, T> {
    unsafe fn pointee_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.typed_bytes.borrow()
    }
}

pub type OwnedRefMutAny<'a> = OwnedRefMut<'a, ()>;
pub type OwnedRefAny<'a> = OwnedRef<'a, ()>;
pub type RefMutAny<'a> = RefMut<'a, ()>;
pub type RefAny<'a> = Ref<'a, ()>;
