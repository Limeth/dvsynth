use super::{AllocationPointer, TypeTrait};
use crate::graph::alloc::Allocator;
use crate::graph::{DynTypeAllocator, NodeIndex};
use crate::node::behaviour::AllocatorHandle;
use std::marker::PhantomData;

/// A common trait for references that allow for shared access.
pub trait RefExt<T: TypeTrait> {
    fn get_ptr(&self) -> AllocationPointer;
}

/// A common trait for references that allow for mutable access.
pub trait RefMutExt<T: TypeTrait>: RefExt<T> {}

// /// A pointer with a dynamic type associated with it.
// pub struct DynTypedPointer {
//     pub(crate) ptr: AllocationPointer,
//     pub(crate) ty: TypeEnum,
// }

/// A refcounted mutable reference to `T`.
pub struct OwnedRefMut<T: TypeTrait> {
    ptr: AllocationPointer,
    node: NodeIndex,
    __marker: PhantomData<T>,
}

// impl<T: TypeTrait> !Send for OwnedRefMut<T> {}
// impl<T: TypeTrait> !Sync for OwnedRefMut<T> {}

// impl<T: TypeTrait + Default> OwnedRefMut<T> {
//     pub fn allocate_default(_handle: AllocatorHandle<'_>) -> Self {
//         Self { ptr: Allocator::get().allocate_any(T::default()), __marker: Default::default() }
//     }
// }

impl<T: TypeTrait> OwnedRefMut<T> {
    fn from_ref_mut<'a>(reference: RefMut<'a, T>, handle: &AllocatorHandle<'a>) -> Self {
        unsafe {
            Allocator::get()
                .refcount_owned_increment(reference.ptr, handle.node)
                .expect("Could not increment the refcount of a RefMut while converting to OwnedRefMut.");
        }

        Self { ptr: reference.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn allocate(descriptor: T::Descriptor, handle: &AllocatorHandle<'_>) -> Self
    where T: DynTypeAllocator {
        Self {
            ptr: Allocator::get().allocate::<T>(descriptor),
            node: handle.node,
            __marker: Default::default(),
        }
    }

    pub fn to_owned_ref(self, handle: &AllocatorHandle<'_>) -> OwnedRef<T> {
        OwnedRef { ptr: self.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn to_mut<'a>(self, _handle: &AllocatorHandle<'a>) -> RefMut<'a, T> {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while converting to RefMut.");
        }

        RefMut { ptr: self.ptr, __marker: Default::default() }
    }

    pub fn to_ref<'a>(self, _handle: &AllocatorHandle<'a>) -> Ref<'a, T> {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while converting to Ref.");
        }

        Ref { ptr: self.ptr, __marker: Default::default() }
    }
}

impl<T: TypeTrait> RefExt<T> for OwnedRefMut<T> {
    fn get_ptr(&self) -> AllocationPointer {
        self.ptr
    }
}

impl<T: TypeTrait> RefMutExt<T> for OwnedRefMut<T> {}

impl<T: TypeTrait> Drop for OwnedRefMut<T> {
    fn drop(&mut self) {
        unsafe {
            Allocator::get()
                .refcount_owned_increment(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRefMut while dropping.");
        }
    }
}

/// A refcounted shared reference to `T`.
#[derive(Clone)]
pub struct OwnedRef<T: TypeTrait> {
    ptr: AllocationPointer,
    node: NodeIndex,
    __marker: PhantomData<T>,
}

// impl<T: TypeTrait> !Send for OwnedRef<T> {}
// impl<T: TypeTrait> !Sync for OwnedRef<T> {}

impl<T: TypeTrait> OwnedRef<T> {
    fn from_ref<'a>(reference: Ref<'a, T>, handle: &AllocatorHandle<'a>) -> Self {
        unsafe {
            Allocator::get()
                .refcount_owned_increment(reference.ptr, handle.node)
                .expect("Could not increment the refcount of a Ref while converting to OwnedRef.");
        }

        Self { ptr: reference.ptr, node: handle.node, __marker: Default::default() }
    }

    pub fn to_ref<'a>(self, handle: &AllocatorHandle<'a>) -> Ref<'a, T> {
        unsafe {
            Allocator::get()
                .refcount_owned_decrement(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while converting to Ref.");
        }

        Ref { ptr: self.ptr, __marker: Default::default() }
    }
}

impl<T: TypeTrait> RefExt<T> for OwnedRef<T> {
    fn get_ptr(&self) -> AllocationPointer {
        self.ptr
    }
}

impl<T: TypeTrait> Drop for OwnedRef<T> {
    fn drop(&mut self) {
        unsafe {
            Allocator::get()
                .refcount_owned_increment(self.ptr, self.node)
                .expect("Could not decrement the refcount of an OwnedRef while dropping.");
        }
    }
}

/// A non-refcounted mutable reference to `T`.
#[repr(transparent)]
pub struct RefMut<'a, T: TypeTrait + 'a> {
    ptr: AllocationPointer,
    __marker: PhantomData<(&'a mut T, *mut T)>,
}

// impl<'a, T: TypeTrait> !Send for RefMut<'a, T> {}
// impl<'a, T: TypeTrait> !Sync for RefMut<'a, T> {}

impl<'a, T: TypeTrait> RefMut<'a, T> {
    pub fn to_owned_mut(self, handle: &AllocatorHandle<'a>) -> OwnedRefMut<T> {
        OwnedRefMut::from_ref_mut(self, handle)
    }

    pub fn to_owned_ref(self, handle: &AllocatorHandle<'a>) -> OwnedRef<T> {
        OwnedRef::from_ref(self.to_ref(handle), handle)
    }

    pub fn to_ref(self, _handle: &AllocatorHandle<'a>) -> Ref<'a, T> {
        Ref { ptr: self.ptr, __marker: Default::default() }
    }
}

impl<'a, T: TypeTrait> RefExt<T> for RefMut<'a, T> {
    fn get_ptr(&self) -> AllocationPointer {
        self.ptr
    }
}

impl<'a, T: TypeTrait> RefMutExt<T> for RefMut<'a, T> {}

/// A non-refcounted shared reference to `T`.
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Ref<'a, T: TypeTrait + 'a> {
    ptr: AllocationPointer,
    __marker: PhantomData<(&'a T, *const T)>,
}

// impl<'a, T: TypeTrait> !Send for Ref<'a, T> {}
// impl<'a, T: TypeTrait> !Sync for Ref<'a, T> {}

impl<'a, T: TypeTrait> Ref<'a, T> {
    pub fn to_owned_ref(self, handle: &AllocatorHandle<'a>) -> OwnedRef<T> {
        OwnedRef::from_ref(self, handle)
    }
}

impl<'a, T: TypeTrait> RefExt<T> for Ref<'a, T> {
    fn get_ptr(&self) -> AllocationPointer {
        self.ptr
    }
}
