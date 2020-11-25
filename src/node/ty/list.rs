use std::fmt::Display;

use crate::graph::alloc::{AllocatedType, AllocationRefGuard, Allocator};
use crate::graph::ListAllocation;
use crate::node::behaviour::AllocatorHandle;

use super::{DowncastFromTypeEnum, DynTypeTrait, RefExt, RefMutExt, TypeEnum, TypeTrait};

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ListType {
    pub item_type: Box<TypeEnum>,
}

impl ListType {
    pub fn new(item_type: impl Into<TypeEnum>) -> Self {
        Self { item_type: Box::new(item_type.into()) }
    }
}

impl Display for ListType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("List<{}>", self.item_type))
    }
}

pub struct ListDescriptor {
    pub item_type: TypeEnum,
}

impl DynTypeTrait for ListType {
    // type DynAllocDispatcher = ListDispatcher;
    type Descriptor = ListDescriptor;
}

pub trait ListRefExt {
    fn len<'a>(&self, handle: &'a mut AllocatorHandle<'a>) -> usize;
    fn read_item<'a>(&self, handle: &'a mut AllocatorHandle<'a>, index: usize) -> Result<&'a [u8], ()>;
}

pub trait ListRefMutExt {
    fn push_item<'a>(&self, handle: &'a mut AllocatorHandle<'a>, data: &[u8]) -> Result<(), ()>;
}

impl<T> ListRefExt for T
where T: RefExt<ListType>
{
    fn len<'a>(&self, handle: &'a mut AllocatorHandle<'a>) -> usize {
        let (data, ty) = handle.deref(self);
        data.data.len()
    }

    fn read_item<'a>(&self, handle: &'a mut AllocatorHandle<'a>, index: usize) -> Result<&'a [u8], ()> {
        let (data, ty) = handle.deref(self);

        if ty.has_safe_binary_representation() {
            let item_size = ty.item_type.value_size();

            if (index + 1) * item_size > data.data.len() {
                Err(())
            } else {
                Ok(&data.data[(index * item_size)..((index + 1) * item_size)])
            }
        } else {
            Err(())
        }
    }
}

impl<T> ListRefMutExt for T
where T: RefMutExt<ListType>
{
    fn push_item<'a>(&self, handle: &'a mut AllocatorHandle<'a>, item: &[u8]) -> Result<(), ()> {
        let (data, ty) = handle.deref_mut(self);

        if ty.has_safe_binary_representation() {
            let item_size = ty.item_type.value_size();

            if item_size == item.len() {
                data.data.extend(item);
                Ok(())
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}

impl From<ListType> for TypeEnum {
    fn from(other: ListType) -> Self {
        TypeEnum::List(other)
    }
}

impl DowncastFromTypeEnum for ListType {
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self> {
        if let TypeEnum::List(inner) = from {
            Some(inner)
        } else {
            None
        }
    }

    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self> {
        if let TypeEnum::List(inner) = from {
            Some(inner)
        } else {
            None
        }
    }
}
