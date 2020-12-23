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
    pub use super::{Ref, RefAny, RefAnyExt, RefMut, RefMutAny, RefMutAnyExt};
}

/// Tracks the number of pointer references.
pub trait Refcounter {
    fn refcount_increment(&self, ptr: AllocationPointer);
    fn refcount_decrement(&self, ptr: AllocationPointer);
}

/// A refcounter that does not track anything.
impl Refcounter for () {
    fn refcount_increment(&self, _ptr: AllocationPointer) {}
    fn refcount_decrement(&self, _ptr: AllocationPointer) {}
}

/// Tracks the number of references stored in the state of a node.
#[derive(Clone, Copy)]
pub struct NodeStateRefcounter(pub NodeIndex);

impl Refcounter for NodeStateRefcounter {
    fn refcount_increment(&self, ptr: AllocationPointer) {
        unsafe { Allocator::get().refcount_owned_increment(ptr, self.0).unwrap() }
    }

    fn refcount_decrement(&self, ptr: AllocationPointer) {
        unsafe { Allocator::get().refcount_owned_decrement(ptr, self.0).unwrap() }
    }
}

pub unsafe fn visit_recursive_postorder<'a>(
    typed_bytes: TypedBytes<'a>,
    visit: &mut dyn FnMut(TypedBytes<'_>),
)
{
    for child in typed_bytes.children() {
        visit_recursive_postorder(child, visit);
    }

    (visit)(typed_bytes.borrow());
}

/// A common trait for references that allow for shared access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait Ref<'a, T: TypeTrait>: RefAny<'a> {
    type Ref<'b, R: TypeTrait>: Ref<'b, R>;
}

/// A common trait for references that allow for mutable access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait RefMut<'a, T: TypeTrait>: Ref<'a, T> + RefMutAny<'a> {
    type RefMut<'b, R: TypeTrait>: RefMut<'b, R>;
}

pub trait RefAny<'a>: Sized {
    unsafe fn refcounter(&self) -> &dyn Refcounter;

    /// Data accessed by dereferencing the pointer.
    ///
    /// `Ref`/`RefMut`: The referenced data.
    /// `OwnedRef`/`OwnedRefMut`: The referenced data.
    unsafe fn rc_and_pointee_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>);

    /// Data accessed by reading the pointer itself.
    ///
    /// `Ref`/`RefMut`: The referenced data.
    /// `OwnedRef`/`OwnedRefMut`: The pointer data itself.
    unsafe fn rc_and_pointing_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        self.rc_and_pointee_typed_bytes()
    }
}

pub trait RefMutAny<'a>: RefAny<'a> {
    unsafe fn refcounter_mut(&mut self) -> &mut dyn Refcounter;

    unsafe fn rc_and_pointee_typed_bytes_mut<'b>(&'b mut self)
        -> (&'b mut dyn Refcounter, TypedBytesMut<'b>);

    unsafe fn rc_and_pointing_typed_bytes_mut<'b>(
        &'b mut self,
    ) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>) {
        self.rc_and_pointee_typed_bytes_mut()
    }

    unsafe fn into_pointee_typed_bytes(self) -> TypedBytesMut<'a>;
}

pub trait RefAnyExt<'a>: RefAny<'a> {
    unsafe fn pointee_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.rc_and_pointee_typed_bytes().1
    }

    unsafe fn pointing_typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.rc_and_pointing_typed_bytes().1
    }

    unsafe fn refcount_increment_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.pointing_typed_bytes(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_increment(ptr);
            }
        });
    }

    unsafe fn refcount_decrement_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.pointing_typed_bytes(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_decrement(ptr);
            }
        });
    }

    unsafe fn refcount_increment_recursive(&self) {
        self.refcount_increment_recursive_for(self.refcounter())
    }

    unsafe fn refcount_decrement_recursive(&self) {
        self.refcount_decrement_recursive_for(self.refcounter())
    }
}

impl<'a, R> RefAnyExt<'a> for R where R: RefAny<'a> {}

pub trait RefMutAnyExt<'a>: RefMutAny<'a> {
    unsafe fn pointee_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        self.rc_and_pointee_typed_bytes_mut().1
    }

    unsafe fn pointing_typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        self.rc_and_pointing_typed_bytes_mut().1
    }
}

impl<'a, R> RefMutAnyExt<'a> for R where R: RefMutAny<'a> {}

// TODO: Consider allowing the lifetime to be a sub-lifetime of 'state?
/// A refcounted mutable reference to `T`.
pub struct OwnedRefMut<'state, T> {
    ptr: AllocationPointer,
    rc: NodeStateRefcounter,
    __marker: PhantomData<&'state T>,
}

impl<'state, T> OwnedRefMut<'state, T> {
    pub(crate) fn into_mut_any(self) -> OwnedRefMutAny<'state> {
        OwnedRefMut { ptr: self.ptr, rc: self.rc, __marker: Default::default() }
    }
}

impl<'state> OwnedRefMut<'state, ()> {
    fn from_ref_mut<'reference, 'invocation, P: UniqueTrait>(
        reference: BorrowedRefMut<'reference, P>,
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
        let result = Self { ptr, rc: NodeStateRefcounter(handle.node), __marker: Default::default() };

        unsafe {
            result.refcounter().refcount_increment(ptr);
        }

        result
    }

    pub fn downcast_mut<'invocation, T: TypeTrait>(self) -> Option<OwnedRefMut<'state, T>>
    where 'state: 'invocation {
        let typed_bytes = unsafe { Allocator::get().deref_mut_ptr(self.ptr) }.unwrap();

        typed_bytes.ty().downcast_ref::<T>().map(|_| OwnedRefMut {
            ptr: self.ptr,
            rc: self.rc,
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
            rc: NodeStateRefcounter(handle.node),
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
            rc: NodeStateRefcounter(handle.node),
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
        OwnedRef { ptr: self.ptr, rc: self.rc, __marker: Default::default() }
    }

    // pub fn to_mut<'invocation>(
    //     self,
    //     _handle: AllocatorHandle<'invocation, 'state>,
    // ) -> BorrowedRefMut<'invocation, T>
    // where
    //     'state: 'invocation,
    // {
    //     let typed_bytes = unsafe {
    //         Allocator::get()
    //             .refcount_owned_decrement(self.ptr, self.node)
    //             .expect("Could not decrement the refcount of an OwnedRef while converting to BorrowedRef.");

    //         Allocator::get().deref_mut_ptr(self.ptr).unwrap()
    //     };

    //     BorrowedRefMut { typed_bytes, rc: &self.rc, __marker: Default::default() }
    // }

    // pub fn to_ref<'invocation>(
    //     self,
    //     _handle: AllocatorHandle<'invocation, 'state>,
    // ) -> BorrowedRef<'invocation, T>
    // {
    //     let typed_bytes = unsafe {
    //         Allocator::get()
    //             .refcount_owned_decrement(self.ptr, self.node)
    //             .expect("Could not decrement the refcount of an OwnedRef while converting to BorrowedRef.");

    //         Allocator::get().deref_ptr(self.ptr).unwrap()
    //     };

    //     BorrowedRef { typed_bytes, __marker: Default::default() }
    // }
}

impl<'a, T> Ref<'a, T> for OwnedRefMut<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = OwnedRef<'b, R>;
}

impl<'a, T> RefMut<'a, T> for OwnedRefMut<'a, T>
where T: TypeTrait
{
    type RefMut<'b, R: TypeTrait> = OwnedRefMut<'b, R>;
}

impl<'a, T> RefAny<'a> for OwnedRefMut<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        &self.rc
    }

    unsafe fn rc_and_pointee_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (&self.rc, Allocator::get().deref_ptr(self.ptr).unwrap())
    }

    unsafe fn rc_and_pointing_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        let typed_bytes = Allocator::get().deref_ptr(self.ptr).unwrap();
        let ptr_ty = Unique::new(typed_bytes.borrow().ty().into_owned()).into();

        (&self.rc, TypedBytes::from(self.ptr.as_bytes(), Cow::Owned(ptr_ty)))
    }
}

impl<'a, T> RefMutAny<'a> for OwnedRefMut<'a, T> {
    unsafe fn refcounter_mut(&mut self) -> &mut dyn Refcounter {
        &mut self.rc
    }

    unsafe fn rc_and_pointee_typed_bytes_mut<'b>(
        &'b mut self,
    ) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>) {
        (&mut self.rc, Allocator::get().deref_mut_ptr(self.ptr).unwrap())
    }

    unsafe fn rc_and_pointing_typed_bytes_mut<'b>(
        &'b mut self,
    ) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>) {
        let typed_bytes = Allocator::get().deref_mut_ptr(self.ptr).unwrap();
        let ptr_ty = Unique::new(typed_bytes.borrow().ty().into_owned()).into();

        (&mut self.rc, TypedBytesMut::from(self.ptr.as_bytes_mut(), Cow::Owned(ptr_ty)))
    }

    unsafe fn into_pointee_typed_bytes(self) -> TypedBytesMut<'a> {
        Allocator::get().deref_mut_ptr(self.ptr).unwrap()
    }
}

impl<'a, T> Drop for OwnedRefMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.refcount_decrement_recursive();
        }
    }
}

// TODO: Consider allowing the lifetime to be a sub-lifetime of 'state?
/// A refcounted shared reference to `T`.
#[derive(Clone)]
pub struct OwnedRef<'state, T> {
    ptr: AllocationPointer,
    rc: NodeStateRefcounter,
    __marker: PhantomData<&'state T>,
}

impl<'state> OwnedRef<'state, ()> {
    fn from_ref<'reference, 'invocation, P: SharedTrait>(
        reference: BorrowedRef<'reference, P>,
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
        let result = Self { ptr, rc: NodeStateRefcounter(handle.node), __marker: Default::default() };

        unsafe {
            result.refcounter().refcount_increment(ptr);
        }

        result
    }

    pub fn downcast_ref<'invocation, T: TypeTrait>(self) -> Option<OwnedRef<'state, T>>
    where 'state: 'invocation {
        let typed_bytes = unsafe { Allocator::get().deref_ptr(self.ptr) }.unwrap();

        typed_bytes.ty().downcast_ref::<T>().map(|_| OwnedRef {
            ptr: self.ptr,
            rc: self.rc,
            __marker: Default::default(),
        })
    }
}

impl<'state, T> OwnedRef<'state, T>
where T: TypeTrait
{
    // pub fn to_ref<'invocation>(
    //     self,
    //     _handle: AllocatorHandle<'invocation, 'state>,
    // ) -> BorrowedRef<'invocation, T>
    // where
    //     'state: 'invocation,
    // {
    //     let typed_bytes = unsafe {
    //         Allocator::get()
    //             .refcount_owned_decrement(self.ptr, self.node)
    //             .expect("Could not decrement the refcount of an OwnedRef while converting to BorrowedRef.");

    //         Allocator::get().deref_ptr(self.ptr).unwrap()
    //     };

    //     BorrowedRef { typed_bytes, __marker: Default::default() }
    // }
}

impl<'state, T> OwnedRef<'state, T> {
    unsafe fn shared_ref_bytes(&self) -> TypedBytes<'_> {
        let typed_bytes = Allocator::get().deref_ptr(self.ptr).unwrap();
        let ptr_ty = Unique::new(typed_bytes.borrow().ty().into_owned()).into();

        TypedBytes::from(self.ptr.as_bytes(), Cow::Owned(ptr_ty))
    }
}

impl<'a, T> Ref<'a, T> for OwnedRef<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = OwnedRef<'b, R>;
}

impl<'a, T> RefAny<'a> for OwnedRef<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        &self.rc
    }

    unsafe fn rc_and_pointee_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (&self.rc, Allocator::get().deref_ptr(self.ptr).unwrap())
    }

    unsafe fn rc_and_pointing_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (&self.rc, self.shared_ref_bytes())
    }
}

impl<'a, T> Drop for OwnedRef<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.refcount_decrement_recursive();
        }
    }
}

/// A non-refcounted mutable reference to `T`.
pub struct BorrowedRefMut<'a, T> {
    pub(crate) typed_bytes: TypedBytesMut<'a>,
    pub(crate) rc: &'a mut dyn Refcounter,
    __marker: PhantomData<(&'a mut T, *mut T)>,
}

impl<'a> BorrowedRefMut<'a, ()> {
    pub unsafe fn from(typed_bytes: TypedBytesMut<'a>, rc: &'a mut dyn Refcounter) -> Self {
        Self { typed_bytes, rc, __marker: Default::default() }
    }

    pub fn downcast_mut<'state: 'a, T: TypeTrait>(self) -> Option<BorrowedRefMut<'a, T>> {
        if self.typed_bytes.borrow().ty().downcast_ref::<T>().is_some() {
            Some(BorrowedRefMut { typed_bytes: self.typed_bytes, rc: self.rc, __marker: Default::default() })
        } else {
            None
        }
    }
}

impl<'a, T> BorrowedRefMut<'a, T>
where T: TypeTrait
{
    pub fn to_ref<'state: 'a>(self, _handle: AllocatorHandle<'a, 'state>) -> BorrowedRef<'a, T> {
        BorrowedRef { typed_bytes: self.typed_bytes.downgrade(), rc: self.rc, __marker: Default::default() }
    }
}

impl<'a, P: UniqueTrait> BorrowedRefMut<'a, P> {
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

impl<'a, P: SharedTrait> BorrowedRefMut<'a, P> {
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

impl<'a, T> Ref<'a, T> for BorrowedRefMut<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = BorrowedRef<'b, R>;
}

impl<'a, T> RefMut<'a, T> for BorrowedRefMut<'a, T>
where T: TypeTrait
{
    type RefMut<'b, R: TypeTrait> = BorrowedRefMut<'b, R>;
}

impl<'a, T> RefAny<'a> for BorrowedRefMut<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        &*self.rc
    }

    unsafe fn rc_and_pointee_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (&*self.rc, self.typed_bytes.borrow())
    }
}

impl<'a, T> RefMutAny<'a> for BorrowedRefMut<'a, T> {
    unsafe fn refcounter_mut(&mut self) -> &mut dyn Refcounter {
        self.rc
    }

    unsafe fn rc_and_pointee_typed_bytes_mut<'b>(
        &'b mut self,
    ) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>) {
        (self.rc, self.typed_bytes.borrow_mut())
    }

    unsafe fn into_pointee_typed_bytes(self) -> TypedBytesMut<'a> {
        self.typed_bytes
    }
}

/// A non-refcounted shared reference to `T`.
#[derive(Clone)]
pub struct BorrowedRef<'a, T> {
    pub(crate) typed_bytes: TypedBytes<'a>,
    pub(crate) rc: &'a dyn Refcounter,
    __marker: PhantomData<(&'a T, *const T)>,
}

impl<'a> BorrowedRef<'a, ()> {
    pub unsafe fn from(typed_bytes: TypedBytes<'a>, rc: &'a dyn Refcounter) -> Self {
        Self { typed_bytes, rc, __marker: Default::default() }
    }

    pub fn downcast_ref<'state: 'a, T: TypeTrait>(self) -> Option<BorrowedRef<'a, T>> {
        if self.typed_bytes.borrow().ty().downcast_ref::<T>().is_some() {
            Some(BorrowedRef { typed_bytes: self.typed_bytes, rc: self.rc, __marker: Default::default() })
        } else {
            None
        }
    }
}

impl<'a, P: SharedTrait> BorrowedRef<'a, P> {
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

impl<'a, T> Ref<'a, T> for BorrowedRef<'a, T>
where T: TypeTrait
{
    type Ref<'b, R: TypeTrait> = BorrowedRef<'b, R>;
}

impl<'a, T> RefAny<'a> for BorrowedRef<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        self.rc
    }

    unsafe fn rc_and_pointee_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (self.rc, self.typed_bytes.borrow())
    }
}

pub type OwnedRefMutAny<'a> = OwnedRefMut<'a, ()>;
pub type OwnedRefAny<'a> = OwnedRef<'a, ()>;
pub type BorrowedRefMutAny<'a> = BorrowedRefMut<'a, ()>;
pub type BorrowedRefAny<'a> = BorrowedRef<'a, ()>;
