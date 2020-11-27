use std::any::Any;
use std::collections::HashSet;
use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryInto;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use crossbeam::deque::Injector;
use crossbeam::deque::Steal;
use crossbeam::epoch::Collector;
use downcast_rs::Downcast;
use lazy_static::lazy_static;
use sharded_slab::{pool::Pool, Clear};

use crate::node::behaviour::AllocatorHandle;
use crate::node::{
    AllocationPointer, DynTypeDescriptor, OwnedRef, OwnedRefMut, Ref, RefExt, RefMut, RefMutExt, TypeEnum,
    TypeTrait,
};

use super::{DynTypeAllocator, NodeIndex, Schedule};

#[derive(Default, Debug)]
pub struct TaskRefCounters {
    pub counters: RwLock<HashMap<NodeIndex, Mutex<TaskRefCounter>>>,
}

/// Counts the changes to refcounts that happen during a single invocation of a task.
/// These changes are then applied to the total refcount after the task has finished executing.
#[derive(Default, Debug)]
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
#[derive(Default)]
pub(crate) struct AllocationCell<T>(T);

impl<T> AllocationCell<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        &self.0 as *const T as *mut T
    }

    pub fn as_ptr(&self) -> *const T {
        &self.0 as *const T
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    pub unsafe fn as_mut<'a>(&self) -> &'a mut T {
        &mut *self.as_mut_ptr()
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.as_ptr()
    }
}

impl<T> From<T> for AllocationCell<T> {
    fn from(other: T) -> Self {
        Self(other)
    }
}

unsafe impl<T> Send for AllocationCell<T> {}
unsafe impl<T> Sync for AllocationCell<T> {}

pub trait AllocatedType = Any + Send + Sync + 'static;

pub(crate) struct AllocationInner {
    ty: TypeEnum,
    data: Box<dyn AllocatedType>,
}

#[derive(Default)]
pub(crate) struct Allocation {
    pub(crate) ptr: AllocationCell<Option<AllocationCell<AllocationInner>>>,
    pub(crate) refcount: AtomicUsize,
    pub(crate) deallocating: AtomicBool,
}

impl Allocation {
    unsafe fn claim<T: DynTypeAllocator>(&self, data: T::DynAlloc, ty: T) {
        let ptr = self.ptr.as_mut();

        assert!(ptr.is_none(), "Allocation already claimed.");

        let ty_enum: TypeEnum = ty.into();
        let data = Box::new(data) as Box<dyn AllocatedType>;
        *ptr = Some(AllocationCell::new(AllocationInner { ty: ty_enum, data }));
        self.refcount.store(0, Ordering::SeqCst);
        self.deallocating.store(false, Ordering::SeqCst);
    }

    unsafe fn free(&self) {
        let ptr = self.ptr.as_mut();

        *ptr = None;
        self.refcount.store(0, Ordering::SeqCst);
        self.deallocating.store(true, Ordering::SeqCst);
    }
}

// pub struct AllocationRefGuard<'a> {
//     ref_guard: sharded_slab::pool::Ref<'a, Allocation>,
// }

// impl<'a> AllocationRefGuard<'a> {
//     fn new(ref_guard: sharded_slab::pool::Ref<'a, Allocation>) -> Self {
//         Self { ref_guard }
//     }
// }

// impl<'a> AllocationRefGuard<'a> {
//     pub fn ty(&self) -> &TypeEnum {
//         // self.ref_guard.ty.as_ref().unwrap()
//         unsafe { &(*self.ref_guard.ptr.as_ref().unwrap().as_ptr()).ty }
//     }

//     pub unsafe fn deref(&self) -> &dyn AllocatedType {
//         &(*self.ref_guard.ptr.as_ref().unwrap().as_ptr()).data
//     }

//     pub unsafe fn deref_mut(&self) -> &mut dyn AllocatedType {
//         &mut (*self.ref_guard.ptr.as_ref().unwrap().as_mut_ptr()).data
//     }
// }

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

#[derive(Default)]
struct Allocations {
    vec: Vec<Pin<Box<Allocation>>>,
    used: usize,
}

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
    allocations: RwLock<Allocations>,
    free_indices: Injector<u64>,
    // collector: Collector,
    // allocations: Pool<Allocation>,
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

    // TODO:
    // * Proper task destructuring
    // * When a node is removed and a new one is created, the index may be the same, but we still
    //   need to be able to signal the removed node to be destructured. Keep generation ID based
    //   on the number of times the node was removed?
    pub(crate) fn prepare_for_schedule(&self, schedule: &Schedule) {
        let mut task_ref_counters = self.task_ref_counters.counters.write().unwrap();
        task_ref_counters.clear();

        for task in &*schedule.tasks {
            task_ref_counters.insert(task.node_index, Default::default());
        }
    }

    /// Allocates the value with refcount set to 1.
    fn allocate_value<'a, T: DynTypeAllocator>(
        &self,
        value: T::DynAlloc,
        ty: T,
        handle: AllocatorHandle<'a>,
    ) -> AllocationPointer
    {
        const EXPAND_BY: usize = 64;

        println!("Allocating...");

        let free_index = loop {
            match self.free_indices.steal() {
                Steal::Success(free_index) => break free_index,
                Steal::Retry => continue,
                Steal::Empty => {
                    let mut allocations = self.allocations.write().unwrap();

                    if allocations.used > allocations.vec.len() {
                        // Already expanded
                        continue;
                    }

                    allocations.vec.reserve(EXPAND_BY);

                    for rel_index in 0..EXPAND_BY {
                        let abs_index =
                            allocations.used.checked_add(rel_index).expect("Allocator slots depleted.");

                        allocations.vec.push(Box::pin(Default::default()));
                        self.free_indices.push(abs_index as u64);
                    }

                    continue;
                }
            }
        };

        let allocations = self.allocations.read().unwrap();
        let allocation = &allocations.vec[free_index as usize];

        unsafe {
            allocation.claim(value, ty);
        }

        let ptr = AllocationPointer { index: free_index };

        unsafe {
            self.refcount_owned_increment(ptr, handle.node).unwrap();
        }

        ptr
    }

    pub fn allocate<'a, T: DynTypeAllocator>(
        &self,
        descriptor: T::Descriptor,
        handle: AllocatorHandle<'a>,
    ) -> AllocationPointer
    {
        let ty = descriptor.get_type();
        self.allocate_value(T::create_value_from_descriptor(descriptor), ty, handle)
    }

    pub fn deallocate(&self, allocation_ptr: AllocationPointer) {
        let allocations = self.allocations.read().unwrap();
        let allocation =
            allocations.vec.get(allocation_ptr.as_usize()).expect("Attempt to free a freed value.");

        if allocation.deallocating.compare_and_swap(false, true, Ordering::SeqCst) {
            // Already deallocated.
            return;
        }

        unsafe {
            allocation.free();
        }

        self.free_indices.push(allocation_ptr.as_u64());
        println!("Deallocated: {:?}", allocation_ptr);
    }

    pub unsafe fn apply_owned_and_output_refcounts(
        &self,
        node: NodeIndex,
        output_delta: (),
    ) -> Result<(), ()>
    {
        let task_ref_counters = self.task_ref_counters.counters.write().map_err(|_| ())?;

        {
            let mut task_ref_counter = task_ref_counters[&node].lock().map_err(|_| ())?;
            // TODO: combine output delta with these
            let altered_ptrs: HashSet<AllocationPointer> =
                task_ref_counter.refcount_deltas.keys().copied().collect();

            for altered_ptr in altered_ptrs {
                let delta = task_ref_counter.refcount_deltas[&altered_ptr];

                self.refcount_global_add(altered_ptr, delta)?;
            }

            task_ref_counter.refcount_deltas.clear();
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
        let allocations = self.allocations.read().unwrap();
        if let Some(allocation) = allocations.vec.get(allocation_ptr.as_usize()) {
            let refcount = &allocation.refcount;

            if delta > 0 {
                refcount.fetch_add(delta as usize, Ordering::SeqCst);
                Ok(false)
            } else {
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

                if refcount_new == 0 {
                    self.deallocate(allocation_ptr);
                    Ok(true)
                } else {
                    // Deallocation was already performed (before_swap == 0) or was not necessary (new > 0).
                    Ok(false)
                }
            }
        } else {
            Err(())
        }
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    ///         Extra caution must be taken to request a correct lifetime 'a.
    unsafe fn deref_ptr<'a>(
        &self,
        allocation_ptr: AllocationPointer,
    ) -> Option<(&'a dyn AllocatedType, &'a TypeEnum)>
    {
        let allocations = self.allocations.read().unwrap();
        allocations.vec.get(allocation_ptr.as_usize()).map(|allocation| {
            let allocation_inner =
                allocation.ptr.as_ref().as_ref().expect("Dereferencing a freed value.").as_ref();

            (allocation_inner.data.as_ref(), &allocation_inner.ty)
        })
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    ///         Extra caution must be taken to request a correct lifetime 'a.
    unsafe fn deref_mut_ptr<'a>(
        &self,
        allocation_ptr: AllocationPointer,
    ) -> Option<(&'a mut dyn AllocatedType, &'a TypeEnum)>
    {
        let allocations = self.allocations.read().unwrap();
        allocations.vec.get(allocation_ptr.as_usize()).map(|allocation| {
            let allocation_inner =
                allocation.ptr.as_ref().as_ref().expect("Dereferencing a freed value.").as_mut();

            (allocation_inner.data.as_mut(), &allocation_inner.ty)
        })
    }

    pub fn deref<'a, T: DynTypeAllocator>(
        &self,
        reference: &dyn RefExt<'a, T>,
    ) -> Option<(&'a T::DynAlloc, &'a T)>
    {
        let (data, ty) = unsafe { self.deref_ptr(reference.get_ptr())? };

        Some((
            data.downcast_ref().expect("Type mismatch when dereferencing."),
            ty.downcast_ref().expect("Type mismatch when dereferencing."),
        ))
    }

    pub fn deref_mut<'a, T: DynTypeAllocator>(
        &self,
        reference: &mut dyn RefMutExt<'a, T>,
    ) -> Option<(&'a mut T::DynAlloc, &'a T)>
    {
        let (data, ty) = unsafe { self.deref_mut_ptr(reference.get_ptr())? };

        Some((
            data.downcast_mut().expect("Type mismatch when dereferencing."),
            ty.downcast_ref().expect("Type mismatch when dereferencing."),
        ))
    }
}
