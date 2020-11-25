use std::any::Any;
use std::collections::HashSet;
use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryInto;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use lazy_static::lazy_static;
use sharded_slab::{pool::Pool, Clear};

use crate::node::{AllocationPointer, RefExt, RefMutExt, TypeEnum, TypeTrait};

use super::{DynTypeAllocator, NodeIndex};

#[derive(Default)]
pub struct TaskRefCounters {
    pub counters: RwLock<HashMap<NodeIndex, Mutex<TaskRefCounter>>>,
}

/// Counts the changes to refcounts that happen during a single invocation of a task.
/// These changes are then applied to the total refcount after the task has finished executing.
#[derive(Default)]
pub struct TaskRefCounter {
    pub refcount_deltas: HashMap<AllocationPointer, isize>,
}

impl AllocationPointer {
    fn new(index: u64) -> Self {
        Self { index }
    }

    fn as_u64(&self) -> u64 {
        self.index
    }

    fn as_usize(&self) -> usize {
        self.index as usize
    }
}

/// Safety: Access safety must be ensured externally by the execution graph.
pub(crate) struct AllocationCell<T: ?Sized>(Box<T>);

impl<T: ?Sized> AllocationCell<T> {
    pub fn new(value: T) -> Self
    where T: Sized {
        Self(Box::new(value))
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    pub unsafe fn as_mut_ptr(&self) -> *mut T {
        self.0.as_ref() as *const T as *mut T
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    pub unsafe fn as_ptr(&self) -> *const T {
        self.0.as_ref() as *const T
    }
}

impl<T: ?Sized> From<Box<T>> for AllocationCell<T> {
    fn from(other: Box<T>) -> Self {
        Self(other)
    }
}

unsafe impl<T: ?Sized> Send for AllocationCell<T> {}
unsafe impl<T: ?Sized> Sync for AllocationCell<T> {}

pub trait AllocatedType = Any + Send + Sync;

#[derive(Default)]
pub(crate) struct Allocation {
    pub(crate) ptr: Option<AllocationCell<dyn AllocatedType>>,
    pub(crate) refcount: AtomicUsize,
    pub(crate) ty: Option<TypeEnum>,
}

impl Clear for Allocation {
    fn clear(&mut self) {
        self.ptr = None;
        self.refcount.store(1, Ordering::SeqCst);
        self.ty = None;
    }
}

pub struct AllocationRefGuard<'a> {
    ref_guard: sharded_slab::pool::Ref<'a, Allocation>,
}

impl<'a> AllocationRefGuard<'a> {
    fn new(ref_guard: sharded_slab::pool::Ref<'a, Allocation>) -> Self {
        Self { ref_guard }
    }
}

impl<'a> AllocationRefGuard<'a> {
    pub fn ty(&self) -> &TypeEnum {
        self.ref_guard.ty.as_ref().unwrap()
    }

    pub unsafe fn deref(&self) -> &dyn AllocatedType {
        &*self.ref_guard.ptr.as_ref().unwrap().as_ptr()
    }

    pub unsafe fn deref_mut(&self) -> &mut dyn AllocatedType {
        &mut *self.ref_guard.ptr.as_ref().unwrap().as_mut_ptr()
    }
}

// pub struct AllocationRefMutGuard<'a> {
//     ref_guard: sharded_slab::pool::Ref<'a, Allocation>,
// }

// impl<'a> AllocationRefMutGuard<'a> {
//     fn new(ref_guard: sharded_slab::pool::Ref<'a, Allocation>) -> Self {
//         Self { ref_guard }
//     }
// }

// impl<'a> AllocationRefMutGuard<'a> {
//     pub fn ty(&self) -> &TypeEnum {
//         self.ref_guard.ty.as_ref().unwrap()
//     }

//     pub unsafe fn deref(&self) -> &dyn AllocatedType {
//         &mut *self.ref_guard.ptr.as_ref().unwrap().as_mut_ptr()
//     }

//     pub unsafe fn deref_mut(&mut self) -> &mut dyn AllocatedType {
//         &mut *self.ref_guard.ptr.as_ref().unwrap().as_mut_ptr()
//     }
// }

// struct AllocatorImpl {
//     allocations: Vec<AllocationCell<Allocation>>,
//     freed_indices: Vec<usize>,
// }

/// The refcount of allocations is tracked in two ways:
/// - globally:
///     Within each allocation, there is a global refcount that is used to determine
///     whether the allocation should be freed.
/// - task-wise:
///     Each task tracks the refcount of all _owned_ references, so that those references
///     can be subtracted when the task is removed. This refcount does **not** track the references
///     written to output channels, which is done separately.
#[derive(Default)]
pub struct Allocator {
    allocations: Pool<Allocation>,
    /// For task-wise refcounting
    task_ref_counters: TaskRefCounters,
    // inner: RwLock<AllocatorImpl>,
}

impl Allocator {
    pub fn get() -> &'static Allocator {
        lazy_static! {
            static ref INSTANCE: Allocator = Allocator::default();
        }
        &*INSTANCE
    }

    /// Allocates the value with refcount set to 1.
    fn allocate_any<T: Any + Send + Sync>(&self, value: T) -> AllocationPointer {
        let mut allocation = self.allocations.create().unwrap();
        allocation.ptr = Some(AllocationCell::from(Box::new(value) as Box<dyn AllocatedType>));

        AllocationPointer { index: allocation.key() as u64 }
    }

    pub fn allocate<T: DynTypeAllocator>(&self, descriptor: T::Descriptor) -> AllocationPointer {
        self.allocate_any(T::create_value_from_descriptor(descriptor))
    }

    pub fn deallocate(&self, allocation_ptr: AllocationPointer) {
        self.allocations.clear(allocation_ptr.as_usize());
    }

    pub unsafe fn apply_owned_and_output_refcounts(
        &self,
        node: NodeIndex,
        output_delta: (),
    ) -> Result<(), ()>
    {
        let task_ref_counters = self.task_ref_counters.counters.read().map_err(|_| ())?;
        let task_ref_counter = task_ref_counters[&node].lock().map_err(|_| ())?;
        // TODO: combine output delta with these
        let altered_ptrs: HashSet<AllocationPointer> =
            task_ref_counter.refcount_deltas.keys().copied().collect();

        for altered_ptr in altered_ptrs {
            let delta = task_ref_counter.refcount_deltas[&altered_ptr];

            if delta != 0 {
                self.refcount_global_add(altered_ptr, delta)?;
            }
        }

        Ok(())
    }

    /// Increment the task-wise refcount of owned values by 1.
    pub unsafe fn refcount_owned_increment(
        &self,
        allocation_ptr: AllocationPointer,
        node: NodeIndex,
    ) -> Result<(), ()>
    {
        self.refcount_owned_add(allocation_ptr, node, 1)
    }

    /// Decrement the task-wise refcount of owned values by 1.
    pub unsafe fn refcount_owned_decrement(
        &self,
        allocation_ptr: AllocationPointer,
        node: NodeIndex,
    ) -> Result<(), ()>
    {
        self.refcount_owned_add(allocation_ptr, node, -1)
    }

    /// Alter the task-wise refcount of owned values.
    pub unsafe fn refcount_owned_add(
        &self,
        allocation_ptr: AllocationPointer,
        node: NodeIndex,
        delta: isize,
    ) -> Result<(), ()>
    {
        let task_ref_counters = self.task_ref_counters.counters.read().map_err(|_| ())?;
        let mut task_ref_counter = task_ref_counters[&node].lock().map_err(|_| ())?;

        match task_ref_counter.refcount_deltas.entry(allocation_ptr) {
            Entry::Occupied(mut entry) => {
                *entry.get_mut() += delta;
            }
            Entry::Vacant(entry) => {
                entry.insert(delta);
            }
        }

        Ok(())
    }

    pub unsafe fn refcount_global_increase(
        &self,
        allocation_ptr: AllocationPointer,
        delta: usize,
    ) -> Result<(), ()>
    {
        if let Ok(delta) = delta.try_into() {
            self.refcount_global_add(allocation_ptr, delta).map(|_| ())
        } else {
            Err(())
        }
    }

    /// Add `delta` to refcount and deallocate, if zero.
    /// Returns `Ok(true)` when the allocation has been freed,
    /// `Ok(false)` resulting refcount is larger than 0,
    /// or `Err` if no such allocation exists.
    pub unsafe fn refcount_global_add(
        &self,
        allocation_ptr: AllocationPointer,
        delta: isize,
    ) -> Result<bool, ()>
    {
        if let Some(allocation) = self.allocations.get(allocation_ptr.as_usize()) {
            let refcount = &allocation.refcount;

            if delta > 0 {
                refcount.fetch_add(delta as usize, Ordering::SeqCst);
                Ok(false)
            } else if delta < 0 {
                let mut refcount_before_swap = refcount.load(Ordering::SeqCst);
                let mut refcount_new;

                loop {
                    refcount_new = refcount_before_swap.saturating_sub((-delta) as usize);
                    let refcount_during_swap =
                        refcount.compare_and_swap(refcount_before_swap, refcount_new, Ordering::SeqCst);

                    if refcount_during_swap == refcount_before_swap {
                        break;
                    } else {
                        refcount_before_swap = refcount_during_swap;
                    }
                }

                if refcount_before_swap > 0 && refcount_new == 0 {
                    self.deallocate(allocation_ptr);
                    Ok(true)
                } else {
                    // Deallocation was already performed (before_swap == 0) or was not necessary (new > 0).
                    Ok(false)
                }
            } else {
                Ok(false)
            }
        } else {
            Err(())
        }
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    pub unsafe fn deref_ptr(&self, allocation_ptr: AllocationPointer) -> Option<AllocationRefGuard<'_>> {
        self.allocations.get(allocation_ptr.as_usize()).map(|ref_guard| AllocationRefGuard::new(ref_guard))
    }

    // /// Safety: Access safety must be ensured externally by the execution graph.
    // pub unsafe fn deref_mut_ptr(
    //     &self,
    //     allocation_ptr: AllocationPointer,
    // ) -> Option<AllocationRefMutGuard<'_>>
    // {
    //     self.allocations.get(allocation_ptr.as_usize()).map(|ref_guard| AllocationRefMutGuard::new(ref_guard))
    // }

    // /// Safety: Access safety must be ensured externally by the execution graph.
    // pub unsafe fn deref<T: TypeTrait>(&self, reference: &dyn RefExt<T>) -> Option<AllocationRefGuard<'_>> {
    //     self.deref_ptr(reference.get_ptr())
    // }

    // /// Safety: Access safety must be ensured externally by the execution graph.
    // pub unsafe fn deref_mut<T: TypeTrait>(
    //     &self,
    //     reference: &dyn RefMutExt<T>,
    // ) -> Option<AllocationRefMutGuard<'_>>
    // {
    //     self.deref_mut_ptr(reference.get_ptr())
    // }
}
