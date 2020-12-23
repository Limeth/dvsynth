use super::{
    BorrowedRefAny, BorrowedRefMutAny, Bytes, DowncastFromTypeEnum, DynTypeDescriptor, DynTypeTrait, Ref,
    RefAnyExt, RefMut, RefMutAny, RefMutAnyExt, SizeRefMutExt, SizedTypeExt, TypeEnum, TypeExt, TypedBytes,
    TypedBytesMut,
};
use crate::graph::ListAllocation;
use crate::util::CowMapExt;
use std::borrow::Cow;
use std::fmt::Display;
use std::ops::Range;

pub mod prelude {
    pub use super::{ListRefExt, ListRefMutExt};
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ListType {
    pub child_ty: Box<TypeEnum>,
}

impl ListType {
    pub fn new(child_ty: impl Into<TypeEnum> + SizedTypeExt) -> Self {
        let child_ty = child_ty.into();
        Self { child_ty: Box::new(child_ty) }
    }

    pub fn new_if_sized(child_ty: impl Into<TypeEnum>) -> Option<Self> {
        let child_ty = child_ty.into();
        child_ty.value_size_if_sized().map(|_| Self { child_ty: Box::new(child_ty) })
    }
}

impl Display for ListType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("List<{}>", self.child_ty))
    }
}

#[derive(Debug)]
pub struct ListDescriptor {
    child_ty: TypeEnum,
}

impl ListDescriptor {
    pub fn new(child_ty: impl Into<TypeEnum> + SizedTypeExt) -> Self {
        Self { child_ty: child_ty.into() }
    }

    pub fn new_if_sized(child_ty: impl Into<TypeEnum>) -> Option<Self> {
        let child_ty = child_ty.into();
        child_ty.value_size_if_sized().map(|_| Self { child_ty })
    }

    pub fn child_ty(&self) -> &TypeEnum {
        &self.child_ty
    }
}

impl DynTypeDescriptor<ListType> for ListDescriptor {
    fn get_type(&self) -> ListType {
        ListType { child_ty: Box::new(self.child_ty.clone()) }
    }
}

impl DynTypeTrait for ListType {
    type Descriptor = ListDescriptor;
    type DynAlloc = ListAllocation;

    fn create_value_from_descriptor(descriptor: Self::Descriptor) -> Self::DynAlloc {
        descriptor.into()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: &Bytes<'a>) -> Vec<TypedBytes<'a>> {
        let value_size = self.child_ty.value_size_if_sized().unwrap();
        let list = data.downcast_ref_unwrap::<ListAllocation>();

        list.data
            .chunks_exact(value_size)
            .map(|chunk| TypedBytes::from(chunk, Cow::Borrowed(self.child_ty.as_ref())))
            .collect()
    }
}

pub trait ListRefExt<'a> {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Result<BorrowedRefAny<'_>, ()>;
}

impl<'a, T> ListRefExt<'a> for T
where T: Ref<'a, ListType>
{
    fn len(&self) -> usize {
        let typed_bytes = unsafe { self.pointee_typed_bytes() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();
        let item_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.bytes().downcast_ref_unwrap::<ListAllocation>();
        list.data.len() / item_size
    }

    fn get(&self, index: usize) -> Result<BorrowedRefAny<'_>, ()> {
        let typed_bytes = unsafe { self.pointee_typed_bytes() };
        let (bytes, ty) = typed_bytes.into();
        let child_ty = ty.map(|ty| {
            let ty = ty.downcast_ref::<ListType>().unwrap();
            ty.child_ty.as_ref()
        });
        let item_size = child_ty.value_size_if_sized().unwrap();
        let list = bytes.downcast_ref_unwrap::<ListAllocation>();

        if (index + 1) * item_size > list.data.len() {
            Err(())
        } else {
            let bytes = &list.data[(index * item_size)..((index + 1) * item_size)];
            Ok(unsafe { BorrowedRefAny::from(TypedBytes::from(bytes, child_ty), self.refcounter()) })
        }
    }
}

pub trait ListRefMutExt<'a> {
    fn remove_range(&mut self, range: Range<usize>) -> Result<(), ()>;
    fn remove(&mut self, index: usize) -> Result<(), ()>;
    fn push<'b>(&mut self, item: impl RefMutAny<'b> + 'b) -> Result<(), ()>;
    fn insert<'b>(&mut self, index: usize, item: impl RefMutAny<'b> + 'b) -> Result<(), ()>;
    fn get_mut(&mut self, index: usize) -> Result<BorrowedRefMutAny<'_>, ()>;

    // API for types with safe binary representation:
    // fn item_range_bytes_mut(&mut self, range: Range<usize>) -> Option<&mut [u8]>;
    // fn item_bytes_mut(&mut self, index: usize) -> Option<&mut [u8]>;
    fn push_item_bytes_with(&mut self, write_bytes: impl FnOnce(&mut [u8])) -> Result<(), ()>;
}

impl<'a, T> ListRefMutExt<'a> for T
where T: RefMut<'a, ListType>
{
    fn get_mut(&mut self, index: usize) -> Result<BorrowedRefMutAny<'_>, ()> {
        let (rc, typed_bytes) = unsafe { self.rc_and_pointee_typed_bytes_mut() };
        let (bytes, ty) = typed_bytes.into();
        let child_ty = ty.map(|ty| {
            let ty = ty.downcast_ref::<ListType>().unwrap();
            ty.child_ty.as_ref()
        });
        let item_size = child_ty.value_size_if_sized().unwrap();
        let list = bytes.downcast_mut_unwrap::<ListAllocation>();

        if (index + 1) * item_size > list.data.len() {
            Err(())
        } else {
            let bytes = &mut list.data[(index * item_size)..((index + 1) * item_size)];
            Ok(unsafe { BorrowedRefMutAny::from(TypedBytesMut::from(bytes, child_ty), rc) })
        }
    }

    fn remove_range(&mut self, range: Range<usize>) -> Result<(), ()> {
        let typed_bytes = unsafe { self.pointee_typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();
        let item_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        let mapped_range = Range { start: range.start * item_size, end: range.end * item_size };

        if mapped_range.end > list.data.len() {
            Err(())
        } else {
            list.data.drain(mapped_range);
            Ok(())
        }
    }

    fn remove(&mut self, index: usize) -> Result<(), ()> {
        self.remove_range(index..(index + 1))
    }

    fn push<'b>(&mut self, mut item: impl RefMutAny<'b> + 'b) -> Result<(), ()> {
        let mut typed_bytes = unsafe { self.pointee_typed_bytes_mut() };
        let item_typed_bytes = unsafe { item.pointing_typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();

        println!("{}", item_typed_bytes.borrow().ty());
        println!("{}", &ty.child_ty);

        if !item_typed_bytes.borrow().ty().is_abi_compatible(&ty.child_ty) {
            return Err(());
        }

        let list = typed_bytes.borrow_mut().bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        let bytes = item_typed_bytes
            .bytes()
            .bytes()
            .expect("Cannot push references to dynamically allocated objects. Use pointers instead.");

        list.data.extend(bytes);

        // Apply refcounts
        unsafe {
            // Increment refcounts in destination.
            item.refcount_increment_recursive_for(self.refcounter());
            // Decrement refcounts in source -- handled by dropping the item.
            drop(item);
        }

        Ok(())
    }

    // fn item_range_bytes_mut(&mut self, range: Range<usize>) -> Option<&mut [u8]> {
    //     let typed_bytes = unsafe { self.pointee_typed_bytes_mut() };
    //     let ty = typed_bytes.borrow().ty().downcast_ref::<ListType>().unwrap();

    //     if !ty.child_ty.has_safe_binary_representation() {
    //         return None;
    //     }

    //     let item_size = ty.child_ty.value_size_if_sized().unwrap();
    //     let list = typed_bytes.bytes_mut().object_mut().unwrap().downcast_mut::<ListAllocation>().unwrap();
    //     let mapped_range = Range { start: range.start * item_size, end: range.end * item_size };

    //     Some(&mut list.data[mapped_range])
    // }

    // fn item_bytes_mut(&mut self, index: usize) -> Option<&mut [u8]> {
    //     self.item_range_bytes_mut(index..(index + 1))
    // }

    fn push_item_bytes_with(&mut self, write_bytes: impl FnOnce(&mut [u8])) -> Result<(), ()> {
        let typed_bytes = unsafe { self.pointee_typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();

        if !ty.child_ty.has_safe_binary_representation() {
            return Err(());
        }

        let child_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        list.data.extend(std::iter::repeat(0).take(child_size));
        self.get_mut(self.len() - 1).map(|mut item| (write_bytes)(item.bytes_mut_if_sized().unwrap()))
    }

    fn insert<'b>(&mut self, index: usize, mut item: impl RefMutAny<'b> + 'b) -> Result<(), ()> {
        let typed_bytes = unsafe { self.pointee_typed_bytes_mut() };
        let item_typed_bytes = unsafe { item.pointing_typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();

        if !item_typed_bytes.borrow().ty().is_abi_compatible(&ty.child_ty) {
            return Err(());
        }

        let item_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        let bytes = item_typed_bytes
            .bytes()
            .bytes()
            .expect("Cannot push references to dynamically allocated objects. Use pointers instead.");
        let tail = list.data.drain((index * item_size)..).collect::<Vec<_>>();

        list.data.extend(bytes.into_iter().copied().chain(tail));

        // Apply refcounts
        unsafe {
            // Increment refcounts in destination.
            item.refcount_increment_recursive_for(self.refcounter());
            // Decrement refcounts in source -- handled by dropping the item.
            drop(item);
        }

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
