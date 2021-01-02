use super::{
    BorrowedRef, BorrowedRefMut, Bytes, BytesMut, DowncastFromTypeEnum, OwnedRefMut, Ref, RefAny, RefAnyExt,
    RefMut, RefMutAny, RefMutAnyExt, SizedTypeExt, TypeDesc, TypeEnum, TypeExt, TypeResolution, TypeTrait,
    TypedBytes, TypedBytesMut,
};
use crate::node::behaviour::AllocatorHandle;
use crate::util::CowMapExt;
use std::borrow::Cow;
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;
use std::marker::PhantomData;

pub mod prelude {
    pub use super::{OptionRefExt, OptionRefMutExt};
}

#[derive(PartialEq, Eq)]
#[repr(u8)]
enum OptionFlags {
    None,
    Some,
}

impl TryFrom<u8> for OptionFlags {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == OptionFlags::None as u8 => Ok(OptionFlags::None),
            _ if value == OptionFlags::Some as u8 => Ok(OptionFlags::Some),
            _ => Err(()),
        }
    }
}

/// A type that is either `None<>`
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct OptionType<T: TypeDesc = !> {
    pub child_ty: Box<TypeEnum>,
    __marker: PhantomData<T>,
}

impl OptionType<!> {
    pub fn from_enum_if_sized(child_ty: impl Into<TypeEnum>) -> Option<Self> {
        let child_ty = child_ty.into();
        child_ty
            .value_size_if_sized()
            .map(|_| Self { child_ty: Box::new(child_ty), __marker: Default::default() })
    }

    pub fn downcast_child<T: TypeDesc>(self) -> Option<OptionType<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            Some(OptionType { child_ty: self.child_ty, __marker: Default::default() })
        } else {
            None
        }
    }

    pub fn downcast_child_ref<T: TypeDesc>(&self) -> Option<&OptionType<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute::<&Self, &OptionType<T>>(self) })
        } else {
            None
        }
    }

    pub fn downcast_child_mut<T: TypeDesc>(&mut self) -> Option<&mut OptionType<T>> {
        if self.child_ty.resolve_ref::<T>().is_some() {
            // Safety: No fields except for the marker `PhantomData` are affected.
            Some(unsafe { std::mem::transmute::<&mut Self, &mut OptionType<T>>(self) })
        } else {
            None
        }
    }
}

impl<T: TypeTrait + SizedTypeExt> OptionType<T> {
    pub fn new(child_ty: T) -> Self {
        Self { child_ty: Box::new(child_ty.into()), __marker: Default::default() }
    }
}

impl<T: TypeDesc> OptionType<T> {
    pub fn upcast(self) -> OptionType<!> {
        OptionType { child_ty: self.child_ty, __marker: Default::default() }
    }
}

impl<T: TypeDesc> OptionType<T> {
    fn get_flags<'a>(&'a self, data: Bytes<'a>) -> OptionFlags {
        let value_size = self.child_ty.value_size_if_sized().unwrap();
        let bytes = data.bytes().unwrap();

        bytes[value_size].try_into().expect("Malformed `OptionType` flags.")
    }

    fn set_flags<'a>(&'a self, data: BytesMut<'a>, flags: OptionFlags) {
        let value_size = self.child_ty.value_size_if_sized().unwrap();
        let bytes = data.bytes_mut().unwrap();

        bytes[value_size] = flags as u8;
    }

    fn get_bytes<'a>(&'a self, data: TypedBytes<'a>) -> Option<TypedBytes<'a>> {
        let value_size = self.child_ty.value_size_if_sized().unwrap();

        match self.get_flags(data.borrow().bytes()) {
            OptionFlags::None => None,
            OptionFlags::Some => Some(
                data.bytes_slice(0..value_size, |ty| &*ty.downcast_ref::<OptionType>().unwrap().child_ty)
                    .unwrap(),
            ),
        }
    }
}

impl<T: TypeDesc> Display for OptionType<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Option<{}>", self.child_ty))
    }
}

unsafe impl<T: TypeDesc> SizedTypeExt for OptionType<T> {
    fn value_size(&self) -> usize {
        // FIXME: use `std::alloc::Layout`s instead
        self.child_ty.value_size_if_sized().unwrap() + 1 // extra byte for flag
    }
}

unsafe impl<T: TypeDesc> TypeExt for OptionType<T> {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.child_ty.is_abi_compatible(&other.child_ty)
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        self.get_bytes(data).into_iter().collect()
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn has_safe_binary_representation(&self) -> bool {
        self.child_ty.has_safe_binary_representation()
    }

    fn is_cloneable(&self) -> bool {
        self.child_ty.is_cloneable()
    }
}

impl<T: TypeDesc> From<OptionType<T>> for TypeEnum {
    fn from(other: OptionType<T>) -> Self {
        TypeEnum::Option(other.upcast())
    }
}

impl<T: TypeDesc> DowncastFromTypeEnum for OptionType<T> {
    fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
    where Self: Sized {
        if let TypeEnum::Option(inner) = from {
            inner.downcast_child::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>> {
        if let TypeEnum::Option(inner) = from {
            inner.downcast_child_ref::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }

    fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>> {
        if let TypeEnum::Option(inner) = from {
            inner.downcast_child_mut::<T>().map(|ty| TypeResolution::Resolved(ty))
        } else {
            None
        }
    }
}

unsafe impl<T: TypeDesc> TypeDesc for OptionType<T> {}
impl<T: TypeDesc> TypeTrait for OptionType<T> {}

pub trait OptionRefExt<'a, C: TypeDesc> {
    fn get(&self) -> Option<BorrowedRef<'_, C>>;
    fn is_some(&self) -> bool;

    fn is_none(&self) -> bool {
        !self.is_some()
    }
}

pub trait OptionRefMutExt<'a, C: TypeDesc> {
    fn get_mut(&mut self) -> Option<BorrowedRefMut<'_, C>>;
    fn take<'state>(&mut self, handle: AllocatorHandle<'_, 'state>) -> Option<OwnedRefMut<'state, C>>;
    fn replace<'state, 'b>(
        &mut self,
        item: impl Into<Option<OwnedRefMut<'b, C>>>,
        handle: AllocatorHandle<'_, 'state>,
    ) -> Result<Option<OwnedRefMut<'state, C>>, ()>;
    fn replace_with_bytes<'state, 'b>(
        &mut self,
        handle: AllocatorHandle<'_, 'state>,
        replace: impl FnOnce(&mut [u8]),
    ) -> Result<Option<OwnedRefMut<'state, C>>, ()>;
}

impl<'a, T, C> OptionRefExt<'a, C> for T
where
    T: Ref<'a, OptionType<C>>,
    C: TypeDesc,
{
    fn get(&self) -> Option<BorrowedRef<'_, C>> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ty = typed_bytes.borrow().ty().map(|ty| ty.downcast_ref::<OptionType>().unwrap());

        match ty.get_flags(typed_bytes.borrow().bytes()) {
            OptionFlags::None => None,
            OptionFlags::Some => {
                let value_size = ty.child_ty.value_size_if_sized().unwrap();
                let inner_typed_bytes = typed_bytes
                    .bytes_slice(0..value_size, |ty| &*ty.downcast_ref::<OptionType>().unwrap().child_ty)
                    .unwrap();

                Some(unsafe { BorrowedRef::from_unchecked_type(inner_typed_bytes) })
            }
        }
    }

    fn is_some(&self) -> bool {
        let typed_bytes = unsafe { self.typed_bytes() };
        let (bytes, ty) = typed_bytes.into();
        let ty = ty.downcast_ref::<OptionType>().unwrap();

        ty.get_flags(bytes) == OptionFlags::Some
    }
}

impl<'a, T, C> OptionRefMutExt<'a, C> for T
where
    T: RefMut<'a, OptionType<C>>,
    C: TypeDesc,
{
    fn get_mut(&mut self) -> Option<BorrowedRefMut<'_, C>> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty().map(|ty| ty.downcast_ref::<OptionType>().unwrap());

        match ty.get_flags(typed_bytes.borrow().bytes()) {
            OptionFlags::None => None,
            OptionFlags::Some => {
                let value_size = ty.child_ty.value_size_if_sized().unwrap();
                let inner_typed_bytes = typed_bytes
                    .bytes_slice_mut(0..value_size, |ty| &*ty.downcast_ref::<OptionType>().unwrap().child_ty)
                    .unwrap();

                Some(unsafe { BorrowedRefMut::from_unchecked_type(inner_typed_bytes) })
            }
        }
    }

    fn take<'state>(&mut self, handle: AllocatorHandle<'_, 'state>) -> Option<OwnedRefMut<'state, C>> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty().map(|ty| ty.downcast_ref::<OptionType>().unwrap());

        match ty.get_flags(typed_bytes.borrow().bytes()) {
            OptionFlags::None => None,
            OptionFlags::Some => {
                let (mut bytes, ty, rc) = typed_bytes.into();
                let option_ty = ty.downcast_ref::<OptionType>().unwrap();

                option_ty.set_flags(bytes.borrow_mut(), OptionFlags::None);

                let mut typed_bytes: TypedBytesMut = (bytes, ty, rc).into();
                let ty = typed_bytes.borrow().ty().map(|ty| ty.downcast_ref::<OptionType>().unwrap());
                let value_size = ty.child_ty.value_size_if_sized().unwrap();
                let inner_typed_bytes = typed_bytes
                    .borrow_mut()
                    .bytes_slice_mut(0..value_size, |ty| &*ty.downcast_ref::<OptionType>().unwrap().child_ty)
                    .unwrap();
                let owned = unsafe {
                    OwnedRefMut::copied_with_unchecked_type_if_sized(inner_typed_bytes.borrow(), handle)
                        .unwrap()
                };

                unsafe {
                    inner_typed_bytes.refcount_decrement_recursive();
                }

                Some(owned)
            }
        }
    }

    fn replace<'state, 'b>(
        &mut self,
        item: impl Into<Option<OwnedRefMut<'b, C>>>,
        handle: AllocatorHandle<'_, 'state>,
    ) -> Result<Option<OwnedRefMut<'state, C>>, ()> {
        let result = self.take(handle);

        if let Some(mut item) = item.into() {
            let typed_bytes = unsafe { self.typed_bytes_mut() };
            let item_typed_bytes = unsafe { item.typed_bytes_mut() };
            let ty = typed_bytes.borrow().ty().map(|ty| ty.downcast_ref::<OptionType>().unwrap());
            let value_size = ty.child_ty.value_size_if_sized().unwrap();

            let mut typed_bytes: TypedBytesMut = {
                let (mut bytes, ty, rc) = typed_bytes.into();
                let option_ty = ty.downcast_ref::<OptionType>().unwrap();

                option_ty.set_flags(bytes.borrow_mut(), OptionFlags::Some);

                (bytes, ty, rc).into()
            };

            let mut inner_typed_bytes = typed_bytes
                .borrow_mut()
                .bytes_slice_mut(0..value_size, |ty| &*ty.downcast_ref::<OptionType>().unwrap().child_ty)
                .unwrap();

            inner_typed_bytes
                .borrow_mut()
                .bytes_mut()
                .bytes_mut()
                .unwrap()
                .copy_from_slice(item_typed_bytes.bytes().bytes().unwrap());

            unsafe {
                inner_typed_bytes.refcount_increment_recursive();
            }
        }

        Ok(result)
    }

    fn replace_with_bytes<'state, 'b>(
        &mut self,
        handle: AllocatorHandle<'_, 'state>,
        replace: impl FnOnce(&mut [u8]),
    ) -> Result<Option<OwnedRefMut<'state, C>>, ()> {
        {
            let typed_bytes = unsafe { self.typed_bytes_mut() };
            let ty = typed_bytes.borrow().ty();

            if ty.value_size_if_sized().is_none() || !ty.has_safe_binary_representation() {
                return Err(());
            }
        }

        let result = self.take(handle);
        let mut typed_bytes: TypedBytesMut = {
            let typed_bytes = unsafe { self.typed_bytes_mut() };
            let (mut bytes, ty, rc) = typed_bytes.into();
            let option_ty = ty.downcast_ref::<OptionType>().unwrap();

            option_ty.set_flags(bytes.borrow_mut(), OptionFlags::Some);

            (bytes, ty, rc).into()
        };
        let bytes = typed_bytes.borrow_mut().bytes_mut().bytes_mut().unwrap();

        (replace)(bytes);

        unsafe {
            typed_bytes.refcount_increment_recursive();
        }

        Ok(result)
    }
}
