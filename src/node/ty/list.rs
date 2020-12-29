use super::{
    BorrowedRef, BorrowedRefMut, Bytes, DowncastFromTypeEnum, DynTypeDescriptor, DynTypeTrait, OwnedRefMut,
    Ref, RefAny, RefAnyExt, RefMut, RefMutAny, RefMutAnyExt, SizeRefMutExt, SizedTypeExt, TypeDesc, TypeEnum,
    TypeExt, TypeResolution, TypeTrait, TypedBytes, TypedBytesMut,
};
use crate::util::CowMapExt;
use std::borrow::Cow;
use std::fmt::Display;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range};

pub mod prelude {
    pub use super::{ListRefExt, ListRefMutExt};
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct ListType<T: TypeDesc = !> {
    pub child_ty: Box<TypeEnum>,
    __marker: PhantomData<T>,
}

impl<T: TypeTrait + SizedTypeExt> ListType<T> {
    pub fn new(child_ty: T) -> Self {
        let child_ty = child_ty.into();
        Self { child_ty: Box::new(child_ty), __marker: Default::default() }
    }
}

impl ListType<!> {
    pub fn new_if_sized(child_ty: impl Into<TypeEnum>) -> Option<Self> {
        let child_ty = child_ty.into();
        child_ty
            .value_size_if_sized()
            .map(|_| Self { child_ty: Box::new(child_ty), __marker: Default::default() })
    }

    pub fn downcast_child<T: TypeDesc>(self) -> Option<ListType<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            Some(ListType { child_ty: self.child_ty, __marker: Default::default() })
        } else {
            None
        }
    }

    pub fn downcast_child_ref<T: TypeDesc>(&self) -> Option<&ListType<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute::<&Self, &ListType<T>>(self) })
        } else {
            None
        }
    }

    pub fn downcast_child_mut<T: TypeDesc>(&mut self) -> Option<&mut ListType<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute::<&mut Self, &mut ListType<T>>(self) })
        } else {
            None
        }
    }
}

impl<T: TypeDesc> ListType<T> {
    pub fn upcast(self) -> ListType<!> {
        ListType { child_ty: self.child_ty, __marker: Default::default() }
    }
}

impl<T: TypeDesc> Display for ListType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("List<{}>", self.child_ty))
    }
}

#[derive(Debug)]
pub struct ListDescriptor<T: TypeDesc = !> {
    pub child_ty: TypeEnum,
    __marker: PhantomData<T>,
}

impl<T: TypeTrait + SizedTypeExt> ListDescriptor<T> {
    pub fn new(child_ty: T) -> Self {
        Self { child_ty: child_ty.into(), __marker: Default::default() }
    }
}

impl ListDescriptor<!> {
    pub fn new_if_sized(child_ty: impl Into<TypeEnum>) -> Option<Self> {
        let child_ty = child_ty.into();
        child_ty.value_size_if_sized().map(|_| Self { child_ty, __marker: Default::default() })
    }
}

impl<T: TypeDesc> ListDescriptor<T> {
    pub fn child_ty(&self) -> &TypeEnum {
        &self.child_ty
    }

    pub fn upcast(self) -> ListDescriptor<!> {
        ListDescriptor { child_ty: self.child_ty, __marker: Default::default() }
    }
}

impl<T: TypeDesc> DynTypeDescriptor<ListType<T>> for ListDescriptor<T> {
    fn get_type(&self) -> ListType<T> {
        ListType { child_ty: Box::new(self.child_ty.clone()), __marker: Default::default() }
    }
}

impl<T: TypeDesc> From<ListDescriptor<T>> for ListAllocation {
    fn from(descriptor: ListDescriptor<T>) -> Self {
        Self {
            item_size: descriptor.child_ty().value_size_if_sized().unwrap(),
            descriptor: descriptor.upcast(),
            data: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct ListAllocation {
    // FIXME: probably not necessary, as type info is stored for each allocation anyway
    pub descriptor: ListDescriptor,
    pub data: Vec<u8>,
    pub item_size: usize,
}

impl ListAllocation {
    pub fn len(&self) -> usize {
        self.data.len() / self.item_size
    }

    pub fn push(&mut self, item: &[u8]) {
        assert_eq!(item.len(), self.item_size);
        self.data.extend_from_slice(item);
    }

    pub fn pop(&mut self) -> Result<(), ()> {
        if self.data.len() > 0 {
            self.data.truncate(self.data.len() - self.item_size);
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn get(&self, index: usize) -> Option<&[u8]> {
        let start_index = index * self.item_size;
        let end_index = (index + 1) * self.item_size;

        if end_index >= self.data.len() {
            Some(&self.data[start_index..end_index])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut [u8]> {
        let start_index = index * self.item_size;
        let end_index = (index + 1) * self.item_size;

        if end_index >= self.data.len() {
            Some(&mut self.data[start_index..end_index])
        } else {
            None
        }
    }
}

impl Deref for ListAllocation {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for ListAllocation {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: TypeDesc> DynTypeTrait for ListType<T> {
    type Descriptor = ListDescriptor<T>;
    type DynAlloc = ListAllocation;

    fn create_value_from_descriptor(descriptor: Self::Descriptor) -> Self::DynAlloc {
        descriptor.into()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        let value_size = self.child_ty.value_size_if_sized().unwrap();
        let (bytes, _, rc) = data.into();
        let list = bytes.downcast_ref_unwrap::<ListAllocation>();

        list.data
            .chunks_exact(value_size)
            .map(|chunk| TypedBytes::from(chunk, Cow::Borrowed(self.child_ty.as_ref()), rc))
            .collect()
    }
}

pub trait ListRefExt<'a, T: TypeDesc> {
    fn len(&self) -> usize;
    fn get(&self, index: usize) -> Result<BorrowedRef<'_, T>, ()>;
}

impl<'a, R, T> ListRefExt<'a, T> for R
where
    R: Ref<'a, ListType<T>>,
    T: TypeDesc,
{
    fn len(&self) -> usize {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();
        let item_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.bytes().downcast_ref_unwrap::<ListAllocation>();
        list.data.len() / item_size
    }

    fn get(&self, index: usize) -> Result<BorrowedRef<'_, T>, ()> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let (bytes, ty, rc) = typed_bytes.into();
        let child_ty = ty.map(|ty| {
            let ty = ty.downcast_ref::<ListType>().unwrap();
            ty.child_ty.as_ref()
        });
        let item_size = child_ty.value_size_if_sized().unwrap();
        let list = bytes.downcast_ref_unwrap::<ListAllocation>();

        if (index + 1) * item_size > list.data.len() {
            Err(())
        } else {
            let range = (index * item_size)..((index + 1) * item_size);
            let bytes = &list.data[range];
            Ok(unsafe { BorrowedRef::from_unchecked_type(TypedBytes::from(bytes, child_ty, rc)) })
        }
    }
}

pub trait ListRefMutExt<'a, T: TypeDesc> {
    // fn remove_range(&mut self, range: Range<usize>) -> Result<(), ()>;
    // fn remove(&mut self, index: usize) -> Result<(), ()>;
    fn push<'b>(&mut self, item: OwnedRefMut<'b, T>) -> Result<(), ()>;
    fn insert<'b>(&mut self, index: usize, item: OwnedRefMut<'b, T>) -> Result<(), ()>;
    fn get_mut(&mut self, index: usize) -> Result<BorrowedRefMut<'_, T>, ()>;

    // API for types with safe binary representation:
    // fn item_range_bytes_mut(&mut self, range: Range<usize>) -> Option<&mut [u8]>;
    // fn item_bytes_mut(&mut self, index: usize) -> Option<&mut [u8]>;
    fn push_item_bytes_with(&mut self, write_bytes: impl FnOnce(&mut [u8])) -> Result<(), ()>;
}

impl<'a, R, T> ListRefMutExt<'a, T> for R
where
    R: RefMut<'a, ListType<T>>,
    T: TypeDesc,
{
    fn get_mut(&mut self, index: usize) -> Result<BorrowedRefMut<'_, T>, ()> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let (bytes, ty, rc) = typed_bytes.into();
        let child_ty = ty.map(|ty| {
            let ty = ty.downcast_ref::<ListType>().unwrap();
            ty.child_ty.as_ref()
        });
        let item_size = child_ty.value_size_if_sized().unwrap();
        let list = bytes.downcast_mut_unwrap::<ListAllocation>();

        if (index + 1) * item_size > list.data.len() {
            Err(())
        } else {
            let range = (index * item_size)..((index + 1) * item_size);
            let bytes = &mut list.data[range];
            Ok(unsafe { BorrowedRefMut::from_unchecked_type(TypedBytesMut::from(bytes, child_ty, rc)) })
        }
    }

    // TODO: refcounting
    //
    // fn remove_range(&mut self, range: Range<usize>) -> Result<(), ()> {
    //     let typed_bytes = unsafe { self.typed_bytes_mut() };
    //     let ty = typed_bytes.borrow().ty();
    //     let ty = ty.downcast_ref::<ListType>().unwrap();
    //     let item_size = ty.child_ty.value_size_if_sized().unwrap();
    //     let list = typed_bytes.bytes_mut().downcast_mut_unwrap::<ListAllocation>();
    //     let mapped_range = Range { start: range.start * item_size, end: range.end * item_size };

    //     if mapped_range.end > list.data.len() {
    //         Err(())
    //     } else {
    //         list.data.drain(mapped_range);
    //         Ok(())
    //     }
    // }

    // fn remove(&mut self, index: usize) -> Result<(), ()> {
    //     self.remove_range(index..(index + 1))
    // }

    fn push<'b>(&mut self, mut item: OwnedRefMut<'b, T>) -> Result<(), ()> {
        let mut typed_bytes = unsafe { self.typed_bytes_mut() };
        let item_typed_bytes = unsafe { item.typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();

        println!("{}", item_typed_bytes.borrow().ty());
        println!("{}", &ty.child_ty);

        if !item_typed_bytes.borrow().ty().is_abi_compatible(&ty.child_ty) {
            return Err(());
        }

        let list = typed_bytes.borrow_mut().bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        let bytes = item_typed_bytes
            .borrow()
            .bytes()
            .bytes()
            .expect("Cannot push references to dynamically allocated objects. Use pointers instead.");

        list.data.extend(bytes);

        // Apply refcounts
        unsafe {
            // Increment refcounts in destination.
            item_typed_bytes.refcount_increment_recursive_for(typed_bytes.refcounter());
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
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();

        if !ty.child_ty.has_safe_binary_representation() {
            return Err(());
        }

        let child_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        list.data.extend(std::iter::repeat(0).take(child_size));
        let mut item_bytes = self.get_mut(self.len() - 1)?;

        (write_bytes)(item_bytes.bytes_mut_if_sized().unwrap());

        unsafe {
            item_bytes.typed_bytes().refcount_increment_recursive();
        }

        Ok(())
    }

    fn insert<'b>(&mut self, index: usize, mut item: OwnedRefMut<'b, T>) -> Result<(), ()> {
        let mut typed_bytes = unsafe { self.typed_bytes_mut() };
        let mut item_typed_bytes = unsafe { item.typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();
        let ty = ty.downcast_ref::<ListType>().unwrap();

        if !item_typed_bytes.borrow().ty().is_abi_compatible(&ty.child_ty) {
            return Err(());
        }

        let item_size = ty.child_ty.value_size_if_sized().unwrap();
        let list = typed_bytes.borrow_mut().bytes_mut().downcast_mut_unwrap::<ListAllocation>();
        let bytes = item_typed_bytes
            .borrow_mut()
            .bytes()
            .bytes()
            .expect("Cannot push references to dynamically allocated objects. Use pointers instead.");
        let tail = list.data.drain((index * item_size)..).collect::<Vec<_>>();

        list.data.extend(bytes.into_iter().copied().chain(tail));

        // Apply refcounts
        unsafe {
            // Increment refcounts in destination.
            item_typed_bytes.refcount_increment_recursive_for(typed_bytes.refcounter());
            // Decrement refcounts in source -- handled by dropping the item.
            drop(item);
        }

        Ok(())
    }
}

impl<T: TypeDesc> From<ListType<T>> for TypeEnum {
    fn from(other: ListType<T>) -> Self {
        TypeEnum::List(other.upcast())
    }
}

impl<T: TypeDesc> DowncastFromTypeEnum for ListType<T> {
    fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
    where Self: Sized {
        if let TypeEnum::List(inner) = from {
            inner.downcast_child::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>> {
        if let TypeEnum::List(inner) = from {
            inner.downcast_child_ref::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>> {
        if let TypeEnum::List(inner) = from {
            inner.downcast_child_mut::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }
}
