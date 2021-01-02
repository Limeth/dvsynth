use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashSet;
use std::collections::{hash_map::Entry, HashMap};
use std::convert::TryInto;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, RwLock};

use crossbeam::deque::Injector;
use crossbeam::deque::Steal;
use lazy_static::lazy_static;

use crate::node::behaviour::AllocatorHandle;
use crate::node::{
    AllocationPointer, Bytes, BytesMut, DynTypeDescriptor, Refcounter, SizedTypeExt, TypeEnum, TypeExt,
    TypeTrait, TypedBytes, TypedBytesMut,
};

use super::{DynTypeTrait, NodeIndex, Schedule};

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
    pub(crate) fn new(index: u64) -> Self {
        Self { index }
    }

    pub(crate) fn as_u64(&self) -> u64 {
        self.index
    }

    pub(crate) fn as_usize(&self) -> usize {
        self.index as usize
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        safe_transmute::transmute_to_bytes(std::slice::from_ref(&self.index))
    }

    pub(crate) fn as_bytes_mut(&mut self) -> &mut [u8] {
        safe_transmute::transmute_to_bytes_mut(std::slice::from_mut(&mut self.index))
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
// pub trait AllocatedType: std::fmt::Debug + Any + Clone + Copy + Send + Sync + 'static {}
// impl<T> AllocatedType for T where T: std::fmt::Debug + Any + Clone + Copy + Send + Sync + 'static {}

#[derive(Debug)]
pub enum AllocationType {
    Bytes(Box<[u8]>),
    Object { ty_name: &'static str, data: Box<dyn AllocatedType> },
}

/// FIXME: An owned equivalent to `Bytes` and `BytesMut`. Come up with better naming.
#[allow(dead_code)]
impl AllocationType {
    pub fn bytes_mut(&mut self) -> Option<&mut [u8]> {
        match self {
            AllocationType::Bytes(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            AllocationType::Bytes(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn object_mut(&mut self) -> Option<&mut dyn AllocatedType> {
        match self {
            AllocationType::Object { data, .. } => Some(data.as_mut()),
            _ => None,
        }
    }

    pub fn object(&self) -> Option<&dyn AllocatedType> {
        match self {
            AllocationType::Object { data, .. } => Some(data.as_ref()),
            _ => None,
        }
    }

    pub fn as_ref(&self) -> Bytes<'_> {
        match self {
            AllocationType::Bytes(inner) => Bytes::Bytes(inner.as_ref()),
            AllocationType::Object { ty_name, data } => Bytes::Object { ty_name, data: data.as_ref() },
        }
    }

    pub fn as_mut(&mut self) -> BytesMut<'_> {
        match self {
            AllocationType::Bytes(inner) => BytesMut::Bytes(inner.as_mut()),
            AllocationType::Object { ty_name, data } => BytesMut::Object { ty_name, data: data.as_mut() },
        }
    }
}

/// FIXME: An owned equivalent to `TypedBytes` and `TypedBytesMut`. Come up with better naming.
#[derive(Debug)]
pub struct AllocationInner {
    ty: TypeEnum,
    inner: AllocationType,
}

#[allow(dead_code)]
impl AllocationInner {
    pub fn new_object<T: DynTypeTrait>(data: T::DynAlloc, ty: T) -> Self {
        assert!(
            TypeId::of::<T::DynAlloc>() != TypeId::of::<[u8]>(),
            "Type `[u8]` may not be allocated as an object. Allocate it as bytes instead."
        );
        let ty_enum: TypeEnum = ty.into();
        let data = Box::new(data) as Box<dyn AllocatedType>;
        let inner = AllocationType::Object { ty_name: std::any::type_name::<T::DynAlloc>(), data };

        Self { ty: ty_enum, inner }
    }

    pub fn new_bytes<T: TypeTrait + SizedTypeExt>(ty: T) -> Self {
        let data: Vec<u8> = std::iter::repeat(0u8).take(ty.value_size()).collect();
        let data: Box<[u8]> = data.into_boxed_slice();
        let inner = AllocationType::Bytes(data);
        let ty_enum: TypeEnum = ty.into();

        Self { ty: ty_enum, inner }
    }

    pub fn from_enum_if_sized(ty: impl Into<TypeEnum>) -> Option<Self> {
        let ty = ty.into();
        let data: Vec<u8> = std::iter::repeat(0u8).take(ty.value_size_if_sized()?).collect();
        let data: Box<[u8]> = data.into_boxed_slice();
        let inner = AllocationType::Bytes(data);

        Some(Self { ty, inner })
    }

    pub fn as_ref<'a>(&'a self, rc: &'a dyn Refcounter) -> TypedBytes<'a> {
        TypedBytes::from(self.inner.as_ref(), Cow::Borrowed(&self.ty), rc)
    }

    pub fn as_mut<'a>(&'a mut self, rc: &'a mut dyn Refcounter) -> TypedBytesMut<'a> {
        TypedBytesMut::from(self.inner.as_mut(), Cow::Borrowed(&self.ty), rc)
    }

    pub fn ty_mut(&mut self) -> &mut TypeEnum {
        &mut self.ty
    }

    pub fn ty(&self) -> &TypeEnum {
        &self.ty
    }

    pub fn inner_mut(&mut self) -> &mut AllocationType {
        &mut self.inner
    }

    pub fn inner(&self) -> &AllocationType {
        &self.inner
    }

    pub fn clone_if_cloneable(&self) -> Option<Self> {
        if self.ty.is_cloneable() {
            match &self.inner {
                AllocationType::Bytes(bytes) => {
                    Some(Self { ty: self.ty.clone(), inner: AllocationType::Bytes(bytes.clone()) })
                }
                AllocationType::Object { .. } => {
                    // TODO: Make it possible to clone opaque objects.
                    todo!("Cloning of opaque objects is not yet implemented.");
                }
            }
        } else {
            None
        }
    }
}

impl From<AllocationInner> for (AllocationType, TypeEnum) {
    fn from(inner: AllocationInner) -> Self {
        (inner.inner, inner.ty)
    }
}

pub(crate) struct Allocation {
    pub(crate) inner: AllocationCell<Option<AllocationCell<AllocationInner>>>,
    pub(crate) refcount: AtomicUsize,
    pub(crate) deallocating: AtomicBool,
}

impl Allocation {
    pub fn new() -> Self {
        Self { inner: Default::default(), refcount: AtomicUsize::new(0), deallocating: AtomicBool::new(true) }
    }
}

impl Allocation {
    unsafe fn claim_with(&self, new_inner: AllocationInner) {
        let inner = self.inner.as_mut();

        assert!(inner.is_none(), "Allocation already claimed.");

        *inner = Some(AllocationCell::new(new_inner));
        self.refcount.store(0, Ordering::SeqCst);
        self.deallocating.store(false, Ordering::SeqCst);
    }

    unsafe fn free(&self) {
        let inner = self.inner.as_mut();

        *inner = None;
        self.refcount.store(0, Ordering::SeqCst);
        self.deallocating.store(true, Ordering::SeqCst);
    }
}

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
            if let Some(task) = task {
                task_ref_counters.insert(task.node_index, Default::default());
            }
        }
    }

    /// Allocates the value with refcount set to 1.
    fn allocate_value(&self, inner: AllocationInner, handle: AllocatorHandle<'_, '_>) -> AllocationPointer {
        const EXPAND_BY: usize = 64;

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
                        let allocation = Allocation::new();

                        allocations.vec.push(Box::pin(allocation));
                        self.free_indices.push(abs_index as u64);
                    }

                    continue;
                }
            }
        };

        let allocations = self.allocations.read().unwrap();
        let allocation = &allocations.vec[free_index as usize];
        let ptr = AllocationPointer { index: free_index };

        unsafe {
            allocation.claim_with(inner);
        }

        unsafe {
            self.refcount_owned_increment(ptr, handle.node).unwrap();
        }

        debugln!("Allocated: {:?}", &ptr);

        ptr
    }

    pub fn allocate_object<T: DynTypeTrait>(
        &self,
        descriptor: T::Descriptor,
        handle: AllocatorHandle<'_, '_>,
    ) -> AllocationPointer {
        let ty = descriptor.get_type();
        let value = T::create_value_from_descriptor(descriptor);
        let inner = AllocationInner::new_object(value, ty);
        self.allocate_value(inner, handle)
    }

    pub fn allocate_bytes<T: TypeTrait + SizedTypeExt>(
        &self,
        ty: T,
        handle: AllocatorHandle<'_, '_>,
    ) -> AllocationPointer {
        let inner = AllocationInner::new_bytes(ty);
        self.allocate_value(inner, handle)
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
        debugln!("Deallocated: {:?}", allocation_ptr);
    }

    pub unsafe fn apply_owned_and_output_refcounts(&self, node: NodeIndex) -> Result<(), ()> {
        let task_ref_counters = self.task_ref_counters.counters.write().map_err(|_| ())?;

        {
            let mut task_ref_counter = task_ref_counters[&node].lock().map_err(|_| ())?;
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
    ) -> Result<(), ()> {
        self.refcount_owned_add(allocation_ptr, node, 1)
    }

    /// Decrement the task-wise refcount of owned values by 1.
    pub unsafe fn refcount_owned_decrement(
        &self,
        allocation_ptr: AllocationPointer,
        node: NodeIndex,
    ) -> Result<(), ()> {
        self.refcount_owned_add(allocation_ptr, node, -1)
    }

    /// Alter the task-wise refcount of owned values.
    pub unsafe fn refcount_owned_add(
        &self,
        allocation_ptr: AllocationPointer,
        node: NodeIndex,
        delta: isize,
    ) -> Result<(), ()> {
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

        debugln!(
            "Owned refcount of pointer {ptr} for node {node:?} changed by {delta}: {result}",
            ptr = allocation_ptr.as_u64(),
            node = node,
            delta = delta,
            result = task_ref_counter.refcount_deltas.get(&allocation_ptr).unwrap(),
        );

        Ok(())
    }

    pub unsafe fn refcount_global_increase(
        &self,
        allocation_ptr: AllocationPointer,
        delta: usize,
    ) -> Result<(), ()> {
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
    ) -> Result<bool, ()> {
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
    pub unsafe fn deref_ptr<'a>(
        &self,
        allocation_ptr: AllocationPointer,
        rc: &'a dyn Refcounter,
    ) -> Option<TypedBytes<'a>> {
        let allocations = self.allocations.read().unwrap();
        allocations.vec.get(allocation_ptr.as_usize()).map(move |allocation| {
            let allocation_inner =
                allocation.inner.as_ref().as_ref().expect("Dereferencing a freed value.").as_ref();

            allocation_inner.as_ref(rc)
        })
    }

    /// Safety: Access safety must be ensured externally by the execution graph.
    ///         Extra caution must be taken to request a correct lifetime 'a.
    pub unsafe fn deref_mut_ptr<'a>(
        &self,
        allocation_ptr: AllocationPointer,
        rc: &'a mut dyn Refcounter,
    ) -> Option<TypedBytesMut<'a>> {
        let allocations = self.allocations.read().unwrap();
        allocations.vec.get(allocation_ptr.as_usize()).map(move |allocation| {
            let allocation_inner =
                allocation.inner.as_ref().as_ref().expect("Dereferencing a freed value.").as_mut();

            allocation_inner.as_mut(rc)
        })
    }

    pub unsafe fn map_type<'a>(
        &self,
        allocation_ptr: AllocationPointer,
        map: impl FnOnce(&mut TypeEnum),
    ) -> Result<(), ()> {
        let allocations = self.allocations.read().unwrap();
        allocations
            .vec
            .get(allocation_ptr.as_usize())
            .map(|allocation| {
                let allocation_inner =
                    allocation.inner.as_ref().as_ref().expect("Dereferencing a freed value.").as_mut();

                (map)(&mut allocation_inner.ty);
            })
            .ok_or(())
    }
}
