use crate::graph::alloc::{AllocatedType, AllocationInner};
use crate::util::CowMapExt;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::mem::Discriminant;
use std::ops::Deref;

pub use array::*;
pub use list::*;
pub use option::*;
pub use primitive::*;
pub use ptr::*;
pub use reference::*;
pub use texture::*;

macro_rules! impl_downcast_from_type_enum {
    ($([$($generics:tt)*])? $variant:ident ($($ty:tt)*)) => {
        impl $(<$($generics)*>)? DowncastFromTypeEnum for $($ty)* {
            fn resolve_from(from: TypeEnum) -> Option<crate::node::ty::TypeResolution<Self, TypeEnum>>
            where Self: Sized {
                if let TypeEnum::$variant(inner) = from {
                    Some(crate::node::ty::TypeResolution::Resolved(inner))
                } else {
                    None
                }
            }

            fn resolve_from_ref(from: &TypeEnum) -> Option<crate::node::ty::TypeResolution<&Self, &TypeEnum>> {
                if let TypeEnum::$variant(inner) = from {
                    Some(crate::node::ty::TypeResolution::Resolved(inner))
                } else {
                    None
                }
            }

            fn resolve_from_mut(from: &mut TypeEnum) -> Option<crate::node::ty::TypeResolution<&mut Self, &mut TypeEnum>> {
                if let TypeEnum::$variant(inner) = from {
                    Some(crate::node::ty::TypeResolution::Resolved(inner))
                } else {
                    None
                }
            }
        }
    };
}

pub mod array;
pub mod list;
pub mod option;
pub mod primitive;
pub mod ptr;
pub mod reference;
pub mod texture;

pub mod prelude {
    pub use super::array::prelude::*;
    pub use super::list::prelude::*;
    pub use super::option::prelude::*;
    pub use super::primitive::prelude::*;
    pub use super::ptr::prelude::*;
    pub use super::reference::prelude::*;
    pub use super::texture::prelude::*;
    pub use super::{
        CloneTypeExt, CloneableTypeExt, DowncastFromTypeEnum, DowncastFromTypeEnumExt,
        SafeBinaryRepresentationTypeExt, SizeRefExt, SizeRefMutExt, SizeTypeExt, SizedRefExt, SizedRefMutExt,
        SizedTypeExt, TypeDesc, TypeExt,
    };
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
#[repr(transparent)]
pub struct AllocationPointer {
    pub(crate) index: u64,
}

pub unsafe fn visit_recursive_postorder<'a>(
    typed_bytes: TypedBytes<'a>,
    visit: &mut dyn FnMut(TypedBytes<'_>),
) {
    for child in typed_bytes.children() {
        visit_recursive_postorder(child, visit);
    }

    (visit)(typed_bytes.borrow());
}

#[derive(Debug, Clone, Copy)]
pub enum Bytes<'a> {
    Bytes(&'a [u8]),
    Object { ty_name: &'static str, data: &'a dyn AllocatedType },
}

impl<'a> Bytes<'a> {
    pub fn borrow(&self) -> Bytes<'_> {
        use self::Bytes::*;
        match self {
            Bytes(ref inner) => Bytes(&**inner),
            Object { ty_name, ref data } => Object { ty_name, data: &**data },
        }
    }

    pub fn bytes_slice<R>(self, range: R) -> Option<Bytes<'a>>
    where [u8]: std::ops::Index<R, Output = [u8]> {
        if let Bytes::Bytes(bytes) = self {
            Some(Bytes::Bytes(&bytes[range]))
        } else {
            None
        }
    }

    pub fn bytes(self) -> Option<&'a [u8]> {
        if let Bytes::Bytes(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn object(self) -> Option<&'a dyn AllocatedType> {
        if let Bytes::Object { data, .. } = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn downcast_ref_unwrap<T: AllocatedType>(self) -> &'a T {
        if let Bytes::Object { ty_name, data } = self {
            data.downcast_ref::<T>().unwrap_or_else(|| {
                panic!("Attempt to downcast type `{}` to `{}`.", ty_name, std::any::type_name::<T>())
            })
        } else {
            panic!(
                "Attempt to downcast type `{}` to `{}`.",
                std::any::type_name::<[u8]>(),
                std::any::type_name::<T>()
            )
        }
    }
}

impl<'a> From<&'a [u8]> for Bytes<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Bytes::Bytes(bytes)
    }
}

#[derive(Debug, Clone)]
pub struct TypedBytes<'a> {
    bytes: Bytes<'a>,
    ty: Cow<'a, TypeEnum>,
    rc: &'a dyn Refcounter,
}

impl<'a> TypedBytes<'a> {
    pub fn from(
        bytes: impl Into<Bytes<'a>>,
        ty: impl Into<Cow<'a, TypeEnum>>,
        rc: &'a dyn Refcounter,
    ) -> Self {
        Self { bytes: bytes.into(), ty: ty.into(), rc }
    }

    pub fn borrow(&self) -> TypedBytes<'_> {
        TypedBytes { bytes: self.bytes.borrow(), ty: Cow::Borrowed(&*self.ty.as_ref()), rc: &*self.rc }
    }

    pub fn bytes(self) -> Bytes<'a> {
        self.bytes
    }

    pub fn ty(self) -> Cow<'a, TypeEnum> {
        self.ty
    }

    pub fn refcounter(self) -> &'a dyn Refcounter {
        self.rc
    }

    pub unsafe fn children(&self) -> Vec<TypedBytes<'_>> {
        self.ty.as_ref().children(self.borrow())
    }

    pub fn map(
        self,
        map_bytes: impl FnOnce(Bytes<'a>) -> Option<Bytes<'a>>,
        map_ty: impl FnOnce(&TypeEnum) -> &TypeEnum,
    ) -> Option<TypedBytes<'a>> {
        let Self { bytes, ty, rc } = self;
        (map_bytes)(bytes).map(move |bytes| Self { bytes, ty: ty.map(map_ty), rc })
    }

    pub fn bytes_slice<R>(
        self,
        range: R,
        map_ty: impl FnOnce(&TypeEnum) -> &TypeEnum,
    ) -> Option<TypedBytes<'a>>
    where
        [u8]: std::ops::Index<R, Output = [u8]>,
    {
        self.map(move |bytes| bytes.bytes_slice(range), map_ty)
    }

    pub fn bytes_slice_with_same_ty<R>(self, range: R) -> Option<TypedBytes<'a>>
    where [u8]: std::ops::Index<R, Output = [u8]> {
        self.bytes_slice(range, |ty| ty)
    }

    pub unsafe fn refcount_increment_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.borrow(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_increment(ptr);
            }
        });
    }

    pub unsafe fn refcount_decrement_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.borrow(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_decrement(ptr);
            }
        });
    }

    pub unsafe fn refcount_increment_recursive(&self) {
        self.refcount_increment_recursive_for(self.borrow().refcounter())
    }

    pub unsafe fn refcount_decrement_recursive(&self) {
        self.refcount_decrement_recursive_for(self.borrow().refcounter())
    }
}

impl<'a> From<TypedBytes<'a>> for (Bytes<'a>, Cow<'a, TypeEnum>) {
    fn from(bytes: TypedBytes<'a>) -> Self {
        (bytes.bytes, bytes.ty)
    }
}

impl<'a> From<TypedBytes<'a>> for (Bytes<'a>, Cow<'a, TypeEnum>, &'a dyn Refcounter) {
    fn from(bytes: TypedBytes<'a>) -> Self {
        (bytes.bytes, bytes.ty, bytes.rc)
    }
}

impl<'a> From<(Bytes<'a>, Cow<'a, TypeEnum>, &'a dyn Refcounter)> for TypedBytes<'a> {
    fn from((bytes, ty, rc): (Bytes<'a>, Cow<'a, TypeEnum>, &'a dyn Refcounter)) -> Self {
        TypedBytes::from(bytes, ty, rc)
    }
}

#[derive(Debug)]
pub enum BytesMut<'a> {
    Bytes(&'a mut [u8]),
    Object { ty_name: &'static str, data: &'a mut dyn AllocatedType },
}

impl<'a> BytesMut<'a> {
    pub fn downgrade(self) -> Bytes<'a> {
        unsafe {
            match self {
                BytesMut::Bytes(inner) => Bytes::Bytes(&*(inner as *const _)),
                BytesMut::Object { ty_name, data } => Bytes::Object { ty_name, data: &*(data as *const _) },
            }
        }
    }

    pub fn borrow_mut(&mut self) -> BytesMut<'_> {
        use self::BytesMut::*;
        match self {
            Bytes(ref mut inner) => Bytes(&mut **inner),
            Object { ty_name, ref mut data } => Object { ty_name, data: &mut **data },
        }
    }

    pub fn borrow(&self) -> Bytes<'_> {
        match self {
            BytesMut::Bytes(ref inner) => Bytes::Bytes(&**inner),
            BytesMut::Object { ty_name, ref data } => Bytes::Object { ty_name, data: &**data },
        }
    }

    pub fn bytes_slice_mut<R>(self, range: R) -> Option<BytesMut<'a>>
    where [u8]: std::ops::IndexMut<R, Output = [u8]> {
        if let BytesMut::Bytes(bytes) = self {
            Some(BytesMut::Bytes(&mut bytes[range]))
        } else {
            None
        }
    }

    pub fn bytes_slice<R>(self, range: R) -> Option<Bytes<'a>>
    where [u8]: std::ops::Index<R, Output = [u8]> {
        if let BytesMut::Bytes(bytes) = self {
            Some(Bytes::Bytes(&bytes[range]))
        } else {
            None
        }
    }

    pub fn bytes(self) -> Option<&'a [u8]> {
        if let BytesMut::Bytes(inner) = self {
            Some(&*inner)
        } else {
            None
        }
    }

    pub fn object(self) -> Option<&'a dyn AllocatedType> {
        if let BytesMut::Object { data, .. } = self {
            Some(&*data)
        } else {
            None
        }
    }

    pub fn bytes_mut(self) -> Option<&'a mut [u8]> {
        if let BytesMut::Bytes(inner) = self {
            Some(inner)
        } else {
            None
        }
    }

    pub fn object_mut(self) -> Option<&'a mut dyn AllocatedType> {
        if let BytesMut::Object { data, .. } = self {
            Some(data)
        } else {
            None
        }
    }

    pub fn downcast_ref_unwrap<T: AllocatedType>(self) -> &'a T {
        if let BytesMut::Object { ty_name, data } = self {
            data.downcast_ref::<T>().unwrap_or_else(|| {
                panic!("Attempt to downcast type `{}` to `{}`.", ty_name, std::any::type_name::<T>())
            })
        } else {
            panic!(
                "Attempt to downcast type `{}` to `{}`.",
                std::any::type_name::<[u8]>(),
                std::any::type_name::<T>()
            )
        }
    }

    pub fn downcast_mut_unwrap<T: AllocatedType>(self) -> &'a mut T {
        if let BytesMut::Object { ty_name, data } = self {
            data.downcast_mut::<T>().unwrap_or_else(|| {
                panic!("Attempt to downcast type `{}` to `{}`.", ty_name, std::any::type_name::<T>())
            })
        } else {
            panic!(
                "Attempt to downcast type `{}` to `{}`.",
                std::any::type_name::<[u8]>(),
                std::any::type_name::<T>()
            )
        }
    }
}

impl<'a> From<&'a mut [u8]> for BytesMut<'a> {
    fn from(bytes: &'a mut [u8]) -> Self {
        BytesMut::Bytes(bytes)
    }
}

#[derive(Debug)]
pub struct TypedBytesMut<'a> {
    bytes: BytesMut<'a>,
    ty: Cow<'a, TypeEnum>,
    rc: &'a mut dyn Refcounter,
}

impl<'a> TypedBytesMut<'a> {
    pub fn from(
        bytes: impl Into<BytesMut<'a>>,
        ty: impl Into<Cow<'a, TypeEnum>>,
        rc: &'a mut dyn Refcounter,
    ) -> Self {
        Self { bytes: bytes.into(), ty: ty.into(), rc }
    }

    pub fn bytes_mut(self) -> BytesMut<'a> {
        self.bytes
    }

    pub fn bytes(self) -> Bytes<'a> {
        self.bytes.downgrade()
    }

    pub fn ty(self) -> Cow<'a, TypeEnum> {
        self.ty
    }

    pub fn refcounter(self) -> &'a dyn Refcounter {
        &*self.rc
    }

    pub fn refcounter_mut(self) -> &'a mut dyn Refcounter {
        self.rc
    }

    pub fn downgrade(self) -> TypedBytes<'a> {
        TypedBytes { bytes: self.bytes.downgrade(), ty: self.ty.clone(), rc: &*self.rc }
    }

    // pub fn borrow<'b>(&'b self) -> TypedBytes<'b> {
    //     // TypedBytes { bytes: self.bytes.borrow(), ty: unsafe { &*(self.ty as *const _) } }
    //     TypedBytes { bytes: self.bytes.borrow(), ty: self.ty.clone() }
    // }

    // pub fn borrow_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
    //     // TypedBytesMut { bytes: self.bytes.borrow_mut(), ty: unsafe { &*(self.ty as *const _) } }
    //     TypedBytesMut { bytes: self.bytes.borrow_mut(), ty: self.ty.clone() }
    // }

    pub fn map(
        self,
        map_bytes: impl FnOnce(BytesMut<'a>) -> Option<BytesMut<'a>>,
        map_ty: impl FnOnce(&TypeEnum) -> &TypeEnum,
    ) -> Option<TypedBytesMut<'a>> {
        let Self { bytes, ty, rc } = self;
        (map_bytes)(bytes).map(move |bytes| Self { bytes, ty: ty.map(map_ty), rc })
    }

    pub fn bytes_slice<R>(
        self,
        range: R,
        map_ty: impl FnOnce(&TypeEnum) -> &TypeEnum,
    ) -> Option<TypedBytes<'a>>
    where
        [u8]: std::ops::Index<R, Output = [u8]>,
    {
        self.downgrade().bytes_slice(range, map_ty)
    }

    pub fn bytes_slice_mut<R>(
        self,
        range: R,
        map_ty: impl FnOnce(&TypeEnum) -> &TypeEnum,
    ) -> Option<TypedBytesMut<'a>>
    where
        [u8]: std::ops::IndexMut<R, Output = [u8]>,
    {
        self.map(move |bytes| bytes.bytes_slice_mut(range), map_ty)
    }

    pub fn bytes_slice_with_same_ty<R>(self, range: R) -> Option<TypedBytes<'a>>
    where [u8]: std::ops::Index<R, Output = [u8]> {
        self.bytes_slice(range, |ty| ty)
    }

    pub fn bytes_slice_mut_with_same_ty<R>(self, range: R) -> Option<TypedBytesMut<'a>>
    where [u8]: std::ops::IndexMut<R, Output = [u8]> {
        self.bytes_slice_mut(range, |ty| ty)
    }

    pub fn borrow_mut(&mut self) -> TypedBytesMut<'_> {
        TypedBytesMut {
            bytes: self.bytes.borrow_mut(),
            ty: Cow::Borrowed(&*self.ty.as_ref()),
            rc: &mut *self.rc,
        }
    }

    pub fn borrow(&self) -> TypedBytes<'_> {
        TypedBytes { bytes: self.bytes.borrow(), ty: Cow::Borrowed(&*self.ty.as_ref()), rc: &*self.rc }
    }

    pub unsafe fn refcount_increment_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.borrow(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_increment(ptr);
            }
        });
    }

    pub unsafe fn refcount_decrement_recursive_for(&self, rc: &dyn Refcounter) {
        visit_recursive_postorder(self.borrow(), &mut |typed_bytes| {
            if let Some(ptr) = crate::ty::ptr::typed_bytes_to_ptr(typed_bytes) {
                rc.refcount_decrement(ptr);
            }
        });
    }

    pub unsafe fn refcount_increment_recursive(&self) {
        self.refcount_increment_recursive_for(self.borrow().refcounter())
    }

    pub unsafe fn refcount_decrement_recursive(&self) {
        self.refcount_decrement_recursive_for(self.borrow().refcounter())
    }
}

impl<'a> From<TypedBytesMut<'a>> for (BytesMut<'a>, Cow<'a, TypeEnum>) {
    fn from(bytes: TypedBytesMut<'a>) -> Self {
        (bytes.bytes, bytes.ty)
    }
}

impl<'a> From<TypedBytesMut<'a>> for (BytesMut<'a>, Cow<'a, TypeEnum>, &'a mut dyn Refcounter) {
    fn from(bytes: TypedBytesMut<'a>) -> Self {
        (bytes.bytes, bytes.ty, bytes.rc)
    }
}

impl<'a> From<(BytesMut<'a>, Cow<'a, TypeEnum>, &'a mut dyn Refcounter)> for TypedBytesMut<'a> {
    fn from((bytes, ty, rc): (BytesMut<'a>, Cow<'a, TypeEnum>, &'a mut dyn Refcounter)) -> Self {
        TypedBytesMut::from(bytes, ty, rc)
    }
}

pub struct DirectInnerRefTypes<T> {
    __marker: PhantomData<T>,
}

pub unsafe trait TypeExt: Sized + PartialEq + Eq + Send + Sync + 'static {
    /// Returns `true`, if the type can be safely cast/reinterpreted as another.
    /// Otherwise returns `false`.
    fn is_abi_compatible(&self, other: &Self) -> bool;

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>>;

    // Type properties.

    /// Returns the size of the associated value, in bytes, or `None`, if unsized.
    fn value_size_if_sized(&self) -> Option<usize> {
        None
    }

    /// Returns `true` of the type is cloneable, otherwise `false`.
    fn is_cloneable(&self) -> bool {
        false
    }

    /// Returns `true`, whether it is possible to let the user read the underlying
    /// binary representation of the associated value. Otherwise returns `false`.
    ///
    /// Pointers and unsized types are both a typical case which is not safe.
    ///
    /// Safety: **safe binary representation implies sized**, or in other words, a type may only
    /// have a safe binary representation if it also is sized. It must not occur that a type has a
    /// safe binary representation but is unsized, that would be an implementation error.
    fn has_safe_binary_representation(&self) -> bool {
        false
    }
}

pub trait SizeTypeExt: TypeExt {
    fn is_sized(&self) -> bool;
}

impl<T> SizeTypeExt for T
where T: TypeExt
{
    fn is_sized(&self) -> bool {
        self.value_size_if_sized().is_some()
    }
}

pub trait CloneTypeExt: TypeExt {
    fn clone_if_cloneable(&self, bytes: Bytes<'_>) -> Option<AllocationInner>;
}

impl<T> CloneTypeExt for T
where T: TypeExt
{
    fn clone_if_cloneable(&self, bytes: Bytes<'_>) -> Option<AllocationInner> {
        if self.is_cloneable() {
            // TODO
            todo!()
        } else {
            None
        }
    }
}

/// Helper traits to implement `TypeExt` properties.
/// These exist to couple conditional logic with type safety by implementing the `TypeExt`
/// properties automatically.
///
/// Implementors of types should either implement these traits (enabling the features)
/// or implement the property trait (usually without having to override the default function
/// implementations).
///
/// For example, say we are declaring a new type `Foo` and we want it to be a sized type, then we
/// would implement `SizedTypeExt` for `Foo`.
/// If, on the other hand, we were declaring a new type `Bar` and we wanted that type to be unsized,
/// then we would implement `SizeType`, leaving the implementation body empty.
// TODO: Add recursive type information using generics with a wildcard.
// For example, `Option<T = Wildcard>` could be used as `Option` or `Option<PrimitiveType>`.
// Then implement helper traits on those types whose child types also implement those traits.
// E.g. if `PrimitiveType: CloneableTypeExt`, then `Option<PrimitiveType>: CloneableTypeExt`.
mod ty_traits {
    use super::*;

    /// A type that implements this trait is guaranteed to be sized.
    /// See [`TypeExt::value_size_if_sized`].
    pub unsafe trait SizedTypeExt: TypeExt {
        fn value_size(&self) -> usize;
    }

    /// A type that implements this trait is guaranteed to be cloneable.
    /// See [`TypeExt::is_cloneable`].
    pub unsafe trait CloneableTypeExt: TypeExt {
        fn clone(&self, bytes: Bytes<'_>) -> AllocationInner {
            self.clone_if_cloneable(bytes).unwrap_or_else(|| {
                panic!("The type `{}` is guaranteed to be cloneable because it implements `CloneableTypeExt`, but its `TypeExt::clone_if_cloneable` returns `None`. This is an implementation error.", std::any::type_name::<Self>());
            })
        }
    }

    /// A type that implements this trait is guaranteed to have a safe binary representation.
    /// See [`TypeExt::has_safe_binary_representation`].
    pub unsafe trait SafeBinaryRepresentationTypeExt: TypeExt + SizedTypeExt {}
}

pub use ty_traits::{CloneableTypeExt, SafeBinaryRepresentationTypeExt, SizedTypeExt};

/// A type descriptor, that can either be a concrete type (implements `TypeTrait`) or a wildcard type `!`.
/// The type `!` stands for any type and can typically be resolved using `downcast` functions into
/// the appropriate concrete type.
///
/// For example, data of type `Unique<List<PrimitiveType>>` can be expressed via all of the
/// following types:
/// * `Unique<List<PrimitiveType>>`
/// * `Unique<List<!>>`
/// * `Unique<!>`
/// * `!`
pub unsafe trait TypeDesc: TypeExt + DowncastFromTypeEnum {
    const WILDCARD: bool = false;

    fn is_wildcard(&self) -> bool {
        Self::WILDCARD
    }
}

unsafe impl TypeExt for ! {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        unreachable!()
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        unreachable!()
    }
}

impl DowncastFromTypeEnum for ! {
    fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
    where Self: Sized {
        Some(TypeResolution::Wildcard(from))
    }

    fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>> {
        Some(TypeResolution::Wildcard(from))
    }

    fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>> {
        Some(TypeResolution::Wildcard(from))
    }
}

unsafe impl TypeDesc for ! {
    const WILDCARD: bool = true;
}

/// Implementors of this trait represent concrete channel types which can be referred to.
pub trait TypeTrait: TypeDesc + Into<TypeEnum> {}

/// The result of a downcast.
pub enum TypeResolution<Resolved, Unresolved> {
    Resolved(Resolved),
    Wildcard(Unresolved),
}

impl<Resolved, Unresolved> TypeResolution<Resolved, Unresolved> {
    pub fn unwrap_resolved(self) -> Resolved {
        if let TypeResolution::Resolved(resolved) = self {
            resolved
        } else {
            panic!("Tried to unwrap `TypeResolution` as `TypeResolution::Resolved`, but the variant does not match.");
        }
    }
}

impl<Resolved> TypeResolution<Resolved, TypeEnum>
where Resolved: Into<TypeEnum>
{
    pub fn into_type_enum(self) -> TypeEnum {
        match self {
            TypeResolution::Resolved(resolved) => resolved.into(),
            TypeResolution::Wildcard(unresolved) => unresolved,
        }
    }
}

impl<Resolved> TypeResolution<&mut Resolved, &mut TypeEnum>
where Resolved: Into<TypeEnum> + Clone
{
    pub fn into_type_enum(self) -> TypeEnum {
        match self {
            TypeResolution::Resolved(resolved) => resolved.clone().into(),
            TypeResolution::Wildcard(unresolved) => unresolved.clone(),
        }
    }
}

impl<Resolved> TypeResolution<&Resolved, &TypeEnum>
where Resolved: Into<TypeEnum> + Clone
{
    pub fn into_type_enum(self) -> TypeEnum {
        match self {
            TypeResolution::Resolved(resolved) => resolved.clone().into(),
            TypeResolution::Wildcard(unresolved) => unresolved.clone(),
        }
    }
}

/// Downcasts the [`TypeEnum`].
pub trait DowncastFromTypeEnum {
    fn resolve_from(from: TypeEnum) -> Option<TypeResolution<Self, TypeEnum>>
    where Self: Sized;
    fn resolve_from_ref(from: &TypeEnum) -> Option<TypeResolution<&Self, &TypeEnum>>;
    fn resolve_from_mut(from: &mut TypeEnum) -> Option<TypeResolution<&mut Self, &mut TypeEnum>>;
}

pub trait DowncastFromTypeEnumExt {
    fn downcast_from(from: TypeEnum) -> Option<Self>
    where Self: Sized;
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self>;
    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self>;
}

impl<T> DowncastFromTypeEnumExt for T
where T: DowncastFromTypeEnum
{
    fn downcast_from(from: TypeEnum) -> Option<Self>
    where Self: Sized {
        Self::resolve_from(from).and_then(|resolution| {
            if let TypeResolution::Resolved(resolved) = resolution {
                Some(resolved)
            } else {
                None
            }
        })
    }

    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self> {
        Self::resolve_from_ref(from).and_then(|resolution| {
            if let TypeResolution::Resolved(resolved) = resolution {
                Some(resolved)
            } else {
                None
            }
        })
    }

    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self> {
        Self::resolve_from_mut(from).and_then(|resolution| {
            if let TypeResolution::Resolved(resolved) = resolution {
                Some(resolved)
            } else {
                None
            }
        })
    }
}

pub trait InitializeWith<T>: Default {
    fn initialize_with(&mut self, descriptor: T);
}

impl InitializeWith<()> for () {
    fn initialize_with(&mut self, _descriptor: ()) {}
}

pub trait DynTypeDescriptor<T: DynTypeTrait<Descriptor = Self>>: Send + Sync + 'static {
    fn get_type(&self) -> T;
}

/// A type that can only be created on the heap.
pub trait DynTypeTrait
where Self: TypeTrait
{
    /// The type to initialize the allocation with.
    type Descriptor: DynTypeDescriptor<Self>;
    type DynAlloc: AllocatedType;

    fn create_value_from_descriptor(descriptor: Self::Descriptor) -> Self::DynAlloc;
    fn is_abi_compatible(&self, other: &Self) -> bool;
    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>>;
}

impl<T> TypeTrait for T where T: DynTypeTrait {}

unsafe impl<T> TypeExt for T
where T: DynTypeTrait
{
    fn is_abi_compatible(&self, other: &Self) -> bool {
        <T as DynTypeTrait>::is_abi_compatible(self, other)
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        <T as DynTypeTrait>::children(self, data)
    }
}

unsafe impl<T> TypeDesc for T where T: DynTypeTrait {}

macro_rules! define_type_enum {
    (@void $($tt:tt)*) => {};

    [
        $($variant:ident($inner:ident) <- $value_for_discriminant:expr),*$(,)?
    ] => {
        #[derive(Debug, Hash, PartialEq, Eq, Clone)]
        pub enum TypeEnum {
            $(
                $variant($inner),
            )*
        }

        impl Display for TypeEnum {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => f.write_fmt(format_args!("{}", inner)),
                    )*
                }
            }
        }

        impl TypeEnum {
            const VARIANT_NAMES: [&'static str; count_tokens!($($variant)*)] = [$(stringify!($variant), )*];

            #[allow(unused_assignments)]
            fn variant_name_of(d: Discriminant<Self>) -> &'static str {
                let mut index = 0;

                loop {
                    $(
                        if (d == std::mem::discriminant(&TypeEnum::$variant($value_for_discriminant))) { break }
                        index += 1;
                    )*
                    unreachable!()
                };

                Self::VARIANT_NAMES[index]
            }

            fn value_size_if_sized_impl(&self) -> Option<usize> {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => inner.value_size_if_sized(),
                    )*
                }
            }

            fn has_safe_binary_representation_impl(&self) -> bool {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => inner.has_safe_binary_representation(),
                    )*
                }
            }

            unsafe fn children_impl<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => TypeExt::children(inner, data),
                    )*
                }
            }

            fn is_cloneable_impl(&self) -> bool {
                use TypeEnum::*;
                match self {
                    $(
                        $variant(inner) => TypeExt::is_cloneable(inner),
                    )*
                }
            }
        }
    }
}

define_type_enum![
    Shared(Shared) <- Shared::new(PrimitiveType::U8).upcast(),
    Unique(Unique) <- Unique::new(PrimitiveType::U8).upcast(),
    Primitive(PrimitiveType) <- PrimitiveType::U8,
    Option(OptionType) <- OptionType::new(PrimitiveType::U8).upcast(),
    // Tuple(Vec<Self>),
    Array(ArrayType) <- ArrayType::single(PrimitiveType::U8),
    List(ListType) <- ListType::new(PrimitiveType::U8).upcast(),
    Texture(TextureType) <- TextureType::new(),
];

impl TypeEnum {
    pub fn resolve<T: DowncastFromTypeEnum + Sized>(self) -> Option<TypeResolution<T, Self>> {
        T::resolve_from(self)
    }

    pub fn resolve_ref<T: DowncastFromTypeEnum>(&self) -> Option<TypeResolution<&T, &Self>> {
        T::resolve_from_ref(self)
    }

    pub fn resolve_mut<T: DowncastFromTypeEnum>(&mut self) -> Option<TypeResolution<&mut T, &mut Self>> {
        T::resolve_from_mut(self)
    }

    pub fn downcast<T: DowncastFromTypeEnum + Sized>(self) -> Option<T> {
        T::downcast_from(self)
    }

    pub fn downcast_ref<T: DowncastFromTypeEnum>(&self) -> Option<&T> {
        T::downcast_from_ref(self)
    }

    pub fn downcast_mut<T: DowncastFromTypeEnum>(&mut self) -> Option<&mut T> {
        T::downcast_from_mut(self)
    }
}

unsafe impl TypeExt for TypeEnum {
    fn is_abi_compatible(&self, other: &Self) -> bool {
        use TypeEnum::*;
        match (self, other) {
            (Array { .. }, _) | (_, Array { .. }) => {
                if self.value_size_if_sized().is_none() || other.value_size_if_sized().is_none() {
                    return false;
                }

                let a = if let Array(array) = self {
                    Cow::Borrowed(array)
                } else {
                    Cow::Owned(ArrayType::single_if_sized(self.clone()).unwrap())
                };
                let b = if let Array(array) = other {
                    Cow::Borrowed(array)
                } else {
                    Cow::Owned(ArrayType::single_if_sized(other.clone()).unwrap())
                };

                return TypeExt::is_abi_compatible(a.as_ref(), b.as_ref());
            }
            (Unique(a), Unique(b)) => return TypeExt::is_abi_compatible(a, b),
            (Shared(a), Shared(b)) => return TypeExt::is_abi_compatible(a, b),
            (Primitive(a), Primitive(b)) => return TypeExt::is_abi_compatible(a, b),
            (List(a), List(b)) => return TypeExt::is_abi_compatible(a, b),
            (Texture(a), Texture(b)) => return TypeExt::is_abi_compatible(a, b),
            (a, b) => {
                debug_assert_ne!(
                    std::mem::discriminant(a),
                    std::mem::discriminant(b),
                    "Missing an implementation of `is_abi_compatible` in `TypeEnum` for the type `{}`.",
                    Self::variant_name_of(std::mem::discriminant(a)),
                );
                false
            }
        }
    }

    unsafe fn children<'a>(&'a self, data: TypedBytes<'a>) -> Vec<TypedBytes<'a>> {
        self.children_impl(data)
    }

    fn value_size_if_sized(&self) -> Option<usize> {
        self.value_size_if_sized_impl()
    }

    fn is_cloneable(&self) -> bool {
        self.is_cloneable_impl()
    }

    fn has_safe_binary_representation(&self) -> bool {
        self.has_safe_binary_representation_impl()
    }
}

pub trait SizedRefMutExt<'a, T: TypeTrait + SizedTypeExt> {
    fn bytes_mut(&mut self) -> Option<&mut [u8]>;
}

impl<'a, R, T> SizedRefMutExt<'a, T> for R
where
    R: RefMut<'a, T>,
    T: TypeTrait + SizedTypeExt,
{
    fn bytes_mut(&mut self) -> Option<&mut [u8]> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };

        if typed_bytes.borrow().ty().has_safe_binary_representation() {
            Some(typed_bytes.bytes_mut().bytes_mut().unwrap())
        } else {
            None
        }
    }
}

pub trait SizedRefExt<'a, T: TypeTrait + SizedTypeExt> {
    fn value_size(&self) -> usize;
    fn bytes(&self) -> Option<&[u8]>;
}

impl<'a, R, T> SizedRefExt<'a, T> for R
where
    R: Ref<'a, T>,
    T: TypeTrait + SizedTypeExt,
{
    fn value_size(&self) -> usize {
        let typed_bytes = unsafe { self.typed_bytes() };
        typed_bytes.ty().value_size_if_sized().unwrap()
    }

    fn bytes(&self) -> Option<&[u8]> {
        let typed_bytes = unsafe { self.typed_bytes() };

        if typed_bytes.borrow().ty().has_safe_binary_representation() {
            Some(typed_bytes.bytes().bytes().unwrap())
        } else {
            None
        }
    }
}

pub trait SizeRefMutExt<'a> {
    fn bytes_mut_if_sized(&mut self) -> Option<&mut [u8]>;
}

impl<'a, R> SizeRefMutExt<'a> for R
where R: RefMutAny<'a>
{
    fn bytes_mut_if_sized(&mut self) -> Option<&mut [u8]> {
        let typed_bytes = unsafe { self.typed_bytes_mut() };
        let ty = typed_bytes.borrow().ty();

        if ty.value_size_if_sized().is_some() && ty.has_safe_binary_representation() {
            Some(typed_bytes.bytes_mut().bytes_mut().unwrap())
        } else {
            None
        }
    }
}

pub trait SizeRefExt<'a> {
    fn value_size_if_sized(&self) -> Option<usize>;
    fn bytes_if_sized(&self) -> Option<&[u8]>;
}

impl<'a, R> SizeRefExt<'a> for R
where R: RefAny<'a>
{
    fn value_size_if_sized(&self) -> Option<usize> {
        let typed_bytes = unsafe { self.typed_bytes() };
        typed_bytes.ty().value_size_if_sized()
    }

    fn bytes_if_sized(&self) -> Option<&[u8]> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ty = typed_bytes.borrow().ty();

        if ty.value_size_if_sized().is_some() && ty.has_safe_binary_representation() {
            Some(typed_bytes.bytes().bytes().as_ref().unwrap())
        } else {
            None
        }
    }
}

pub trait AssignRefMutExt {
    fn swap<'b>(&mut self, from: &mut impl RefMutAny<'b>) -> Result<(), ()>;
    fn assign<'b>(&mut self, from: impl RefMutAny<'b>) -> Result<(), ()>;
}

impl<'a, R> AssignRefMutExt for R
where R: RefMutAny<'a>
{
    fn swap<'b>(&mut self, from: &mut impl RefMutAny<'b>) -> Result<(), ()> {
        {
            let typed_bytes_a = unsafe { self.typed_bytes_mut() };
            let typed_bytes_b = unsafe { from.typed_bytes_mut() };

            // Swapped types must be compatible
            if !typed_bytes_a.borrow().ty().as_ref().is_abi_compatible(typed_bytes_b.borrow().ty().as_ref()) {
                return Err(());
            }

            // Ensure types are sized
            if typed_bytes_a.borrow().ty().value_size_if_sized().is_none()
                || typed_bytes_b.borrow().ty().value_size_if_sized().is_none()
            {
                return Err(());
            }

            unsafe {
                typed_bytes_a.refcount_increment_recursive_for(typed_bytes_b.borrow().refcounter());
                typed_bytes_b.refcount_increment_recursive_for(typed_bytes_a.borrow().refcounter());
                typed_bytes_a.refcount_decrement_recursive();
                typed_bytes_b.refcount_decrement_recursive();
            }
        }

        let mut typed_bytes_a = unsafe { self.typed_bytes_mut() };
        let mut typed_bytes_b = unsafe { from.typed_bytes_mut() };

        // Swap values
        if let (Some(a), Some(b)) = (
            typed_bytes_a.borrow_mut().bytes_mut().bytes_mut(),
            typed_bytes_b.borrow_mut().bytes_mut().bytes_mut(),
        ) {
            assert_eq!(a.len(), b.len());

            a.iter_mut().zip(b.iter_mut()).for_each(|(a, b)| {
                std::mem::swap(a, b);
            });

            return Ok(());
        } else {
            unreachable!()
        }
    }

    fn assign<'b>(&mut self, mut from: impl RefMutAny<'b>) -> Result<(), ()> {
        self.swap(&mut from)
    }
}
