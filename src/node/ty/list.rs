use std::fmt::Display;

use crate::graph::alloc::{AllocatedType, Allocator};
use crate::graph::ListAllocation;
use crate::node::behaviour::AllocatorHandle;
use crate::ty::prelude::*;

use super::{
    DowncastFromTypeEnum, DynTypeDescriptor, DynTypeTrait, RefAny, RefExt, RefMutAny, RefMutExt, TypeEnum,
    TypeExt, TypedBytes, TypedBytesMut,
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
    fn get(self, index: usize) -> Result<RefAny<'a>, ()>;
}

pub trait ListRefMutExt<'a> {
    fn remove(self, index: usize) -> Result<RefMutAny<'a>, ()>;
    fn push<'b>(self, item: impl RefMutDynExt<'b> + 'b) -> Result<(), ()>;
    fn insert<'b>(self, index: usize, item: impl RefMutDynExt<'b> + 'b) -> Result<(), ()>;
}

impl<'a, T> ListRefExt<'a> for T
where T: RefExt<'a, ListType>
{
    fn len(self) -> usize {
        let typed_bytes = unsafe { self.typed_bytes() };
        let list = typed_bytes.bytes().object().unwrap().downcast_ref::<ListAllocation>().unwrap();
        let ty = typed_bytes.ty().downcast_ref::<ListType>().unwrap();
        list.data.len() / ty.item_type.value_size()
    }

    fn get(self, index: usize) -> Result<RefAny<'a>, ()> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let list = typed_bytes.bytes().object().unwrap().downcast_ref::<ListAllocation>().unwrap();
        let ty = typed_bytes.ty().downcast_ref::<ListType>().unwrap();
        let item_size = ty.item_type.value_size();

        if (index + 1) * item_size > list.data.len() {
            Err(())
        } else {
            let bytes = &list.data[(index * item_size)..((index + 1) * item_size)];
            Ok(unsafe { RefAny::from(TypedBytes::from(bytes, ty.item_type.as_ref())) })
        }
    }
}

impl<'a, T> ListRefMutExt<'a> for T
where T: RefMutExt<'a, ListType>
{
    fn remove(self, index: usize) -> Result<RefMutAny<'a>, ()> {
        // let typed_bytes = unsafe { self.typed_bytes_mut() };
        // let list = typed_bytes.bytes_mut().object().unwrap().downcast_mut::<ListAllocation>().unwrap();
        // let ty = typed_bytes.ty().downcast_ref::<ListType>().unwrap();
        // let item_size = ty.item_type.value_size();

        // if (index + 1) * item_size > list.data.len() {
        //     Err(())
        // } else {
        //     let bytes_range = (index * item_size)..((index + 1) * item_size);
        //     let bytes: Vec<u8> = list.data.drain(bytes_range).collect();
        //     Ok(unsafe { RefMutAny::from(TypedBytesMut::from(bytes, ty.item_type.as_ref())) })
        // }
        todo!()
    }

    fn push<'b>(self, item: impl RefMutDynExt<'b> + 'b) -> Result<(), ()> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let item_typed_bytes = unsafe { item.typed_bytes() };
        let ty = typed_bytes.ty.downcast_ref::<ListType>().unwrap();

        if !item_typed_bytes.ty().is_abi_compatible(&ty.item_type) {
            return Err(());
        }

        let list = typed_bytes.bytes_mut().object_mut().unwrap().downcast_mut::<ListAllocation>().unwrap();
        let bytes = item_typed_bytes
            .bytes()
            .bytes()
            .expect("Cannot push references to dynamically allocated objects. Use pointers instead.");

        list.data.extend(bytes);
        Ok(())
    }

    fn insert<'b>(self, index: usize, item: impl RefMutDynExt<'b> + 'b) -> Result<(), ()> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let item_typed_bytes = unsafe { item.typed_bytes() };
        let ty = typed_bytes.ty.downcast_ref::<ListType>().unwrap();

        if !item_typed_bytes.ty().is_abi_compatible(&ty.item_type) {
            return Err(());
        }

        let list = typed_bytes.bytes_mut().object_mut().unwrap().downcast_mut::<ListAllocation>().unwrap();
        let item_size = ty.item_type.value_size();
        let bytes = item_typed_bytes
            .bytes()
            .bytes()
            .expect("Cannot push references to dynamically allocated objects. Use pointers instead.");
        let tail = list.data.drain((index * item_size)..).collect::<Vec<_>>();

        list.data.extend(bytes.into_iter().copied().chain(tail));
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
