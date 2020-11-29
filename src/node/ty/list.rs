use std::fmt::Display;

use crate::graph::alloc::{AllocatedType, Allocator};
use crate::graph::ListAllocation;
use crate::node::behaviour::AllocatorHandle;
use crate::ty::prelude::*;

use super::{
    DowncastFromTypeEnum, DynTypeDescriptor, DynTypeTrait, RefAny, RefExt, RefMutAny, RefMutExt, TypeEnum,
    TypeExt,
};

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
    fn get_item(self, index: usize) -> Result<RefAny<'a>, ()>;
}

pub trait ListRefMutExt<'a> {
    fn get_item_mut(self, index: usize) -> Result<RefMutAny<'a>, ()>;
    fn push_item<'b>(self, item: impl RefMutDynExt<'b>) -> Result<(), ()>;
}

impl<'a, T> ListRefExt<'a> for T
where T: RefExt<'a, ListType>
{
    fn len(self) -> usize {
        let (data, ty) = unsafe { self.get_inner().deref_ref().unwrap() };
        data.data.len() / ty.item_type.value_size()
    }

    fn get_item(self, index: usize) -> Result<RefAny<'a>, ()> {
        let (data, ty) = unsafe { self.get_inner().deref_ref().unwrap() };
        let item_size = ty.item_type.value_size();

        if (index + 1) * item_size > data.data.len() {
            Err(())
        } else {
            let bytes = &data.data[(index * item_size)..((index + 1) * item_size)];
            Ok(unsafe { RefAny::from(bytes, ty.item_type.as_ref()) })
        }
    }
}

impl<'a, T> ListRefMutExt<'a> for T
where T: RefMutExt<'a, ListType>
{
    fn get_item_mut(self, index: usize) -> Result<RefMutAny<'a>, ()> {
        let (data, ty) = unsafe { self.get_inner().deref_ref().unwrap() };
        let item_size = ty.item_type.value_size();

        if (index + 1) * item_size > data.data.len() {
            Err(())
        } else {
            let bytes = &mut data.data[(index * item_size)..((index + 1) * item_size)];
            Ok(unsafe { RefMutAny::from(bytes, ty.item_type.as_ref()) })
        }
    }

    fn push_item<'b>(self, item: impl RefMutDynExt<'b>) -> Result<(), ()> {
        let (data, ty) = unsafe { self.get_mut_inner().deref_ref_mut().unwrap() };
        let item_size = ty.item_type.value_size();

        if !item.ty_equals(&ty.item_type) {
            return Err(());
        }

        data.data.extend(unsafe { item.bytes() });
        Ok(())
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
