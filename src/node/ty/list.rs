use std::fmt::Display;

use crate::graph::alloc::{AllocatedType, Allocator};
use crate::graph::ListAllocation;
use crate::node::behaviour::AllocatorHandle;
use crate::ty::prelude::*;

use super::{DowncastFromTypeEnum, DynTypeDescriptor, DynTypeTrait, RefExt, RefMutExt, TypeEnum, TypeExt};

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

impl DynTypeDescriptor<ListType> for ListDescriptor {
    fn get_type(&self) -> ListType {
        ListType { item_type: Box::new(self.item_type.clone()) }
    }
}

impl DynTypeTrait for ListType {
    type Descriptor = ListDescriptor;
    type DynAlloc = ListAllocation;

    fn create_value_from_descriptor(descriptor: Self::Descriptor) -> Self::DynAlloc {
        descriptor.into()
    }
}

pub trait ListRefExt<'a> {
    fn len(self) -> usize;
    fn read_item(self, index: usize) -> Result<&'a [u8], ()>;
}

pub trait ListRefMutExt<'a> {
    fn push_item(self, data: &[u8]) -> Result<(), ()>;
}

impl<'a, T> ListRefExt<'a> for T
where T: RefExt<'a, ListType>
{
    fn len(self) -> usize {
        let (data, ty) = self.get_inner().deref_ref().unwrap();
        data.data.len() / ty.item_type.value_size()
    }

    fn read_item(self, index: usize) -> Result<&'a [u8], ()> {
        let (data, ty) = self.get_inner().deref_ref().unwrap();

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

impl<'a, T> ListRefMutExt<'a> for T
where T: RefMutExt<'a, ListType>
{
    fn push_item(self, item: &[u8]) -> Result<(), ()> {
        let (data, ty) = self.get_mut_inner().deref_ref_mut().unwrap();

        if ty.item_type.has_safe_binary_representation() {
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
