use super::{
    AllocationPointer, CloneTypeExt, CloneableTypeExt, Shared, SharedTrait, SizedTypeExt, TypeDesc, TypeEnum,
    TypeExt, TypeTrait, TypedBytes, TypedBytesMut, Unique, UniqueTrait,
};
use crate::graph::alloc::{AllocationInner, Allocator};
use crate::graph::NodeIndex;
use crate::node::behaviour::AllocatorHandle;
use crate::node::ty::DynTypeTrait;
use crate::util::SmallBoxedSlice;
use byteorder::{LittleEndian, ReadBytesExt};
use smallvec::{smallvec, Array, SmallVec};
use std::borrow::Cow;
use std::io::Cursor;
use std::marker::PhantomData;

pub mod prelude {
    pub use super::{Ref, RefAny, RefAnyExt, RefMut, RefMutAny, RefMutAnyExt};
}

/// Heap-allocated byte slice large enough to hold an [`AllocationPointer`].
pub type OwnedBoxedBytes = SmallBoxedSlice<[u8; 8]>;

impl<A: Array<Item = u8>> From<AllocationPointer> for SmallBoxedSlice<A> {
    fn from(ptr: AllocationPointer) -> Self {
        Self::from(ptr.as_bytes())
    }
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
) {
    for child in typed_bytes.children() {
        visit_recursive_postorder(child, visit);
    }

    (visit)(typed_bytes.borrow());
}

/// A common trait for references that allow for shared access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait Ref<'a, T: TypeDesc>: RefAny<'a> {}

pub trait RefExt<'a, T: TypeDesc>: Ref<'a, T> {
    fn clone_if_cloneable<'invocation, 'state>(
        self,
        handle: AllocatorHandle<'a, 'state>,
    ) -> Option<OwnedRefMut<'state, T>>
    where
        'invocation: 'a,
        'state: 'invocation;

    fn clone<'invocation, 'state>(self, handle: AllocatorHandle<'a, 'state>) -> OwnedRefMut<'state, T>
    where
        'invocation: 'a,
        'state: 'invocation,
        T: CloneableTypeExt;
}

impl<'a, T: TypeDesc, R> RefExt<'a, T> for R
where R: Ref<'a, T>
{
    fn clone_if_cloneable<'invocation, 'state>(
        self,
        handle: AllocatorHandle<'a, 'state>,
    ) -> Option<OwnedRefMut<'state, T>>
    where
        'invocation: 'a,
        'state: 'invocation,
    {
        OwnedRefMut::clone_from_if_cloneable(self, handle)
    }

    fn clone<'invocation, 'state>(self, handle: AllocatorHandle<'a, 'state>) -> OwnedRefMut<'state, T>
    where
        'invocation: 'a,
        'state: 'invocation,
        T: CloneableTypeExt,
    {
        OwnedRefMut::clone_from(self, handle)
    }
}

/// A common trait for references that allow for mutable access.
/// The lifetime `'a` denotes how long the underlying data may be accessed for.
pub trait RefMut<'a, T: TypeDesc>: Ref<'a, T> + RefMutAny<'a> {}

pub trait RefAny<'a>: Sized {
    unsafe fn refcounter(&self) -> &dyn Refcounter;

    /// Data accessed by dereferencing the pointer.
    ///
    /// `Ref`/`RefMut`: The referenced data.
    /// `OwnedRef`/`OwnedRefMut`: The referenced data.
    unsafe fn rc_and_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>);
}

pub trait RefMutAny<'a>: RefAny<'a> {
    unsafe fn refcounter_mut(&mut self) -> &mut dyn Refcounter;

    unsafe fn rc_and_typed_bytes_mut<'b>(&'b mut self) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>);

    // FIXME: separate into an OwnedAny?
    // unsafe fn into_typed_bytes(self) -> TypedBytesMut<'a>;
}

pub trait RefAnyExt<'a>: RefAny<'a> {
    unsafe fn typed_bytes<'b>(&'b self) -> TypedBytes<'b> {
        self.rc_and_typed_bytes().1
    }

    unsafe fn refcount_increment_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.typed_bytes(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_increment(ptr);
            }
        });
    }

    unsafe fn refcount_decrement_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.typed_bytes(), &mut |typed_bytes| {
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
    unsafe fn typed_bytes_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        self.rc_and_typed_bytes_mut().1
    }
}

impl<'a, R> RefMutAnyExt<'a> for R where R: RefMutAny<'a> {}

// TODO: Consider allowing the lifetime to be a sub-lifetime of 'state?
/// A refcounted mutable reference to `T`.
pub struct OwnedRefMut<'state, T: TypeDesc = !> {
    ty: TypeEnum,
    bytes: OwnedBoxedBytes,
    rc: NodeStateRefcounter,
    __marker: PhantomData<&'state T>,
}

impl<'state> OwnedRefMut<'state, !> {
    pub fn downcast<'invocation, T: TypeDesc>(self) -> Option<OwnedRefMut<'state, T>>
    where 'state: 'invocation {
        if self.ty.downcast_ref::<T>().is_some() {
            Some(unsafe { self.reinterpret() })
        } else {
            None
        }
    }

    /// Safety: A zeroed buffer may not be a valid value for the provided type and must be
    ///         initialized properly.
    pub unsafe fn zeroed_from_enum_if_sized(
        ty: TypeEnum,
        handle: AllocatorHandle<'_, 'state>,
    ) -> Option<Self> {
        ty.value_size_if_sized().map(|size| OwnedRefMut {
            bytes: smallvec![0; size].into(),
            ty,
            rc: NodeStateRefcounter(handle.node),
            __marker: Default::default(),
        })
    }

    pub fn copied_from(typed_bytes: TypedBytes<'_>, handle: AllocatorHandle<'_, 'state>) -> Option<Self> {
        let (bytes_src, ty) = typed_bytes.into();
        unsafe {
            Self::zeroed_from_enum_if_sized(ty.into_owned(), handle).map(|mut owned| {
                owned.bytes.copy_from_slice(bytes_src.bytes().unwrap());
                owned
            })
        }
    }
}

impl<'state, T: TypeTrait> OwnedRefMut<'state, T> {
    pub fn upcast(self) -> OwnedRefMut<'state> {
        unsafe { self.reinterpret() }
    }
}

impl<'state, T: TypeDesc> OwnedRefMut<'state, T> {
    /// Reinterpret the referred value to be of type `R`. Does not affect the lifetime.
    ///
    /// Safety: The method may only be called if one of the following holds:
    /// * `T = ()` and `Self::ty` downcasts to `R`;
    /// * `R = ()`.
    pub(crate) unsafe fn reinterpret<R: TypeDesc>(self) -> OwnedRefMut<'state, R> {
        // Safety: Source and target types are of the same layout, the type `T`
        // is only used in `PhantomData`.
        std::mem::transmute(self)
    }

    /// Safety: A zeroed buffer may not be a valid value for the provided type and must be
    ///         initialized properly.
    ///         Additionally, it must be possible to downcast the type of `typed_bytes` to
    ///         the generic type `T`.
    pub unsafe fn zeroed_from_enum_with_unchecked_type_if_sized(
        ty: TypeEnum,
        handle: AllocatorHandle<'_, 'state>,
    ) -> Option<Self> {
        ty.value_size_if_sized().map(|size| OwnedRefMut {
            bytes: smallvec![0; size].into(),
            ty,
            rc: NodeStateRefcounter(handle.node),
            __marker: Default::default(),
        })
    }

    /// Safety: It must be possible to downcast the type of `typed_bytes` to
    ///         the generic type `T`.
    pub unsafe fn copied_with_unchecked_type_if_sized(
        typed_bytes: TypedBytes<'_>,
        handle: AllocatorHandle<'_, 'state>,
    ) -> Option<Self> {
        let (bytes_src, ty) = typed_bytes.into();
        Self::zeroed_from_enum_with_unchecked_type_if_sized(ty.into_owned(), handle).map(|mut owned| {
            owned.bytes.copy_from_slice(bytes_src.bytes().unwrap());
            owned
        })
    }

    fn clone_from_if_cloneable<'reference, 'invocation>(
        reference: impl Ref<'reference, T>,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> Option<Self>
    where
        'invocation: 'reference,
        'state: 'invocation,
    {
        let typed_bytes = unsafe { reference.typed_bytes() };
        let (bytes, ty) = typed_bytes.into();
        let (bytes, ty) = ty.clone_if_cloneable(bytes)?.into();

        let result = Self {
            ty,
            bytes: bytes.bytes()?.into(),
            rc: NodeStateRefcounter(handle.node),
            __marker: Default::default(),
        };

        unsafe {
            result.refcount_increment_recursive();
        }

        Some(result)
    }

    fn clone_from<'reference, 'invocation>(
        reference: impl Ref<'reference, T>,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> Self
    where
        'invocation: 'reference,
        'state: 'invocation,
        T: CloneableTypeExt,
    {
        Self::clone_from_if_cloneable(reference, handle)
            .expect("Could not clone the value of a cloneable type.")
    }
}

impl<'state, T: TypeDesc> OwnedRefMut<'state, Unique<T>> {
    pub fn allocate_object<'invocation>(
        descriptor: T::Descriptor,
        handle: AllocatorHandle<'invocation, 'state>,
    ) -> Self
    where
        'state: 'invocation,
        T: DynTypeTrait,
    {
        let ptr = Allocator::get().allocate_object::<T>(descriptor, handle);
        let typed_bytes = unsafe { Allocator::get().deref_ptr(ptr) }.unwrap();
        let ty = typed_bytes.ty().into_owned();

        OwnedRefMut {
            ty: Unique::from_enum(ty).into(),
            bytes: ptr.into(),
            rc: NodeStateRefcounter(handle.node),
            __marker: Default::default(),
        }
    }

    /// Safety: The value's type must have been changed to `Shared` prior to calling this method.
    pub(crate) unsafe fn into_shared(self) -> OwnedRefMut<'state, Shared<T>> {
        self.upcast().downcast().unwrap()
    }
}

impl<'a, T: TypeDesc> Ref<'a, T> for OwnedRefMut<'a, T> {}

impl<'a, T: TypeDesc> RefMut<'a, T> for OwnedRefMut<'a, T> {}

impl<'a, T: TypeDesc> RefAny<'a> for OwnedRefMut<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        &self.rc
    }

    unsafe fn rc_and_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (&self.rc, TypedBytes::from(&*self.bytes, Cow::Borrowed(&self.ty)))
    }
}

impl<'a, T: TypeDesc> RefMutAny<'a> for OwnedRefMut<'a, T> {
    unsafe fn refcounter_mut(&mut self) -> &mut dyn Refcounter {
        &mut self.rc
    }

    unsafe fn rc_and_typed_bytes_mut<'b>(&'b mut self) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>) {
        (&mut self.rc, TypedBytesMut::from(&mut *self.bytes, Cow::Borrowed(&self.ty)))
    }

    // unsafe fn into_typed_bytes(self) -> TypedBytesMut<'a> {
    //     Allocator::get().deref_mut_ptr(self.ptr).unwrap()
    // }
}

impl<'a, T: TypeDesc> Drop for OwnedRefMut<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.refcount_decrement_recursive();
        }
    }
}

/// A non-refcounted mutable reference to `T`.
pub struct BorrowedRefMut<'a, T: TypeDesc = !> {
    pub(crate) typed_bytes: TypedBytesMut<'a>,
    pub(crate) rc: &'a mut dyn Refcounter,
    __marker: PhantomData<(&'a mut T, *mut T)>,
}

impl<'a, T: TypeDesc> BorrowedRefMut<'a, T> {
    /// Safety: It must be possible to downcast `typed_bytes` to the generic type `T`.
    pub unsafe fn from_unchecked_type(typed_bytes: TypedBytesMut<'a>, rc: &'a mut dyn Refcounter) -> Self {
        Self { typed_bytes, rc, __marker: Default::default() }
    }
}

impl<'a> BorrowedRefMut<'a, !> {
    pub unsafe fn from(typed_bytes: TypedBytesMut<'a>, rc: &'a mut dyn Refcounter) -> Self {
        Self { typed_bytes, rc, __marker: Default::default() }
    }

    pub fn downcast_mut<'state: 'a, T: TypeDesc>(self) -> Option<BorrowedRefMut<'a, T>> {
        if self.typed_bytes.borrow().ty().downcast_ref::<T>().is_some() {
            Some(BorrowedRefMut { typed_bytes: self.typed_bytes, rc: self.rc, __marker: Default::default() })
        } else {
            None
        }
    }
}

impl<'a, T: TypeDesc> BorrowedRefMut<'a, T> {
    pub fn to_ref<'state: 'a>(self, _handle: AllocatorHandle<'a, 'state>) -> BorrowedRef<'a, T> {
        BorrowedRef { typed_bytes: self.typed_bytes.downgrade(), rc: self.rc, __marker: Default::default() }
    }
}

impl<'a, T: TypeDesc> BorrowedRefMut<'a, T> {
    pub fn upcast(self) -> BorrowedRefMut<'a> {
        BorrowedRefMut { typed_bytes: self.typed_bytes, rc: self.rc, __marker: Default::default() }
    }
}

impl<'a, T: TypeDesc> Ref<'a, T> for BorrowedRefMut<'a, T> {}

impl<'a, T: TypeDesc> RefMut<'a, T> for BorrowedRefMut<'a, T> {}

impl<'a, T: TypeDesc> RefAny<'a> for BorrowedRefMut<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        &*self.rc
    }

    unsafe fn rc_and_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (&*self.rc, self.typed_bytes.borrow())
    }
}

impl<'a, T: TypeDesc> RefMutAny<'a> for BorrowedRefMut<'a, T> {
    unsafe fn refcounter_mut(&mut self) -> &mut dyn Refcounter {
        self.rc
    }

    unsafe fn rc_and_typed_bytes_mut<'b>(&'b mut self) -> (&'b mut dyn Refcounter, TypedBytesMut<'b>) {
        (self.rc, self.typed_bytes.borrow_mut())
    }

    // unsafe fn into_typed_bytes(self) -> TypedBytesMut<'a> {
    //     self.typed_bytes
    // }
}

/// A non-refcounted shared reference to `T`.
#[derive(Clone)]
pub struct BorrowedRef<'a, T: TypeDesc = !> {
    pub(crate) typed_bytes: TypedBytes<'a>,
    pub(crate) rc: &'a dyn Refcounter,
    __marker: PhantomData<(&'a T, *const T)>,
}

impl<'a, T: TypeDesc> BorrowedRef<'a, T> {
    /// Safety: It must be possible to downcast `typed_bytes` to the generic type `T`.
    pub unsafe fn from_unchecked_type(typed_bytes: TypedBytes<'a>, rc: &'a dyn Refcounter) -> Self {
        Self { typed_bytes, rc, __marker: Default::default() }
    }
}

impl<'a> BorrowedRef<'a, !> {
    pub unsafe fn from(typed_bytes: TypedBytes<'a>, rc: &'a dyn Refcounter) -> Self {
        Self { typed_bytes, rc, __marker: Default::default() }
    }

    pub fn downcast_ref<'state: 'a, T: TypeDesc>(self) -> Option<BorrowedRef<'a, T>> {
        if self.typed_bytes.borrow().ty().downcast_ref::<T>().is_some() {
            Some(BorrowedRef { typed_bytes: self.typed_bytes, rc: self.rc, __marker: Default::default() })
        } else {
            None
        }
    }
}

impl<'a, T: TypeDesc> BorrowedRef<'a, T> {
    pub fn upcast(self) -> BorrowedRef<'a> {
        BorrowedRef { typed_bytes: self.typed_bytes, rc: self.rc, __marker: Default::default() }
    }
}

impl<'a, T: TypeDesc> Ref<'a, T> for BorrowedRef<'a, T> {}

impl<'a, T: TypeDesc> RefAny<'a> for BorrowedRef<'a, T> {
    unsafe fn refcounter(&self) -> &dyn Refcounter {
        self.rc
    }

    unsafe fn rc_and_typed_bytes<'b>(&'b self) -> (&'b dyn Refcounter, TypedBytes<'b>) {
        (self.rc, self.typed_bytes.borrow())
    }
}
