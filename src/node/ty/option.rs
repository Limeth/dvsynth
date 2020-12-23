use crate::util::CowMapExt;
use std::borrow::Cow;
use std::convert::{TryFrom, TryInto};
use std::fmt::Display;

use super::{
    BorrowedRefAny, BorrowedRefMutAny, Bytes, BytesMut, DowncastFromTypeEnum, Ref, RefAnyExt, RefMut,
    RefMutAny, RefMutAnyExt, SizedTypeExt, TypeEnum, TypeExt, TypeTrait, TypedBytes, TypedBytesMut,
};

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
pub struct OptionType {
    pub item_type: Box<TypeEnum>,
}

impl OptionType {
    pub fn new(item_type: impl Into<TypeEnum> + SizedTypeExt) -> Self {
        Self { item_type: Box::new(item_type.into()) }
    }

    pub fn new_if_sized(item_type: impl Into<TypeEnum>) -> Option<Self> {
        let item_type = item_type.into();
        item_type.value_size_if_sized().map(|_| Self { item_type: Box::new(item_type) })
    }

    fn get_flags<'a>(&'a self, data: Bytes<'a>) -> OptionFlags {
        let value_size = self.item_type.value_size_if_sized().unwrap();
        let bytes = data.bytes().unwrap();

        bytes[value_size].try_into().expect("Malformed `OptionType` flags.")
    }

    fn get_bytes<'a>(&'a self, data: Bytes<'a>) -> Option<TypedBytes<'a>> {
        let value_size = self.item_type.value_size_if_sized().unwrap();

        match self.get_flags(data) {
            OptionFlags::None => None,
            OptionFlags::Some => {
                let bytes = data.bytes().unwrap();
                Some(TypedBytes::from(&bytes[0..value_size - 1], Cow::Borrowed(self.item_type.as_ref())))
            }
        }
    }
}

impl Display for OptionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Option<{}>", self.item_type))
    }
}

unsafe impl SizedTypeExt for OptionType {
    fn value_size(&self) -> usize {
        // FIXME: use `std::alloc::Layout`s instead
        self.item_type.value_size_if_sized().unwrap() + 1 // extra byte for flag
    }
}

unsafe impl TypeExt for OptionType {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        self.item_type.is_abi_compatible(&other.item_type)
    }

    unsafe fn children<'a>(&'a self, data: Bytes<'a>) -> Vec<TypedBytes<'a>> {
        self.get_bytes(data).into_iter().collect()
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        Some(self.value_size())
    }

    fn has_safe_binary_representation(&self) -> bool {
        self.item_type.has_safe_binary_representation()
    }

    fn is_cloneable(&self) -> bool {
        self.item_type.is_cloneable()
    }
}

impl From<OptionType> for TypeEnum {
    fn from(other: OptionType) -> Self {
        TypeEnum::Option(other)
    }
}

impl_downcast_from_type_enum!(Option(OptionType));

impl TypeTrait for OptionType {}

pub trait OptionRefExt<'a> {
    fn get(&self) -> Option<BorrowedRefAny<'_>>;
    fn is_some(&self) -> bool;

    fn is_none(&self) -> bool {
        !self.is_some()
    }
}

pub trait OptionRefMutExt<'a> {
    fn get_mut(&mut self) -> Option<BorrowedRefMutAny<'_>>;
    fn set<'b, R>(&mut self, item: impl Into<Option<R>>) -> Result<(), ()>
    where R: RefMutAny<'b> + 'b;
}

impl<'a, T> OptionRefExt<'a> for T
where T: Ref<'a, OptionType>
{
    fn get(&self) -> Option<BorrowedRefAny<'_>> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let (bytes, ty) = typed_bytes.into();
        let ty = ty.map(|ty| ty.downcast_ref::<OptionType>().unwrap());

        match ty.get_flags(bytes) {
            OptionFlags::None => None,
            OptionFlags::Some => {
                let value_size = ty.item_type.value_size_if_sized().unwrap();
                let child_ty = ty.map(|ty| ty.item_type.as_ref());
                let bytes = bytes.bytes().unwrap();
                let child_typed_bytes = TypedBytes::from(&bytes[0..value_size - 1], child_ty);
                Some(unsafe { BorrowedRefAny::from(child_typed_bytes, self.refcounter()) })
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

impl<'a, T> OptionRefMutExt<'a> for T
where T: RefMut<'a, OptionType>
{
    fn get_mut(&mut self) -> Option<BorrowedRefMutAny<'_>> {
        let (rc, typed_bytes) = unsafe { self.rc_and_typed_bytes_mut() };
        let (bytes, ty) = typed_bytes.into();
        let ty = ty.map(|ty| ty.downcast_ref::<OptionType>().unwrap());

        match ty.get_flags(bytes.borrow()) {
            OptionFlags::None => None,
            OptionFlags::Some => {
                let value_size = ty.item_type.value_size_if_sized().unwrap();
                let child_ty = ty.map(|ty| ty.item_type.as_ref());
                let bytes = bytes.bytes_mut().unwrap();
                let child_typed_bytes = TypedBytesMut::from(&mut bytes[0..value_size - 1], child_ty);
                Some(unsafe { BorrowedRefMutAny::from(child_typed_bytes, rc) })
            }
        }
    }

    fn set<'b, R>(&mut self, item: impl Into<Option<R>>) -> Result<(), ()>
    where R: RefMutAny<'b> + 'b {
        todo!()
    }
}
