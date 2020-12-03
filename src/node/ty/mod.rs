use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use downcast_rs::Downcast;

pub use array::*;
pub use list::*;
pub use primitive::*;
pub use ptr::*;
pub use reference::*;
pub use texture::*;

use crate::graph::alloc::{AllocatedType, AllocationInner, Allocator};

use super::behaviour::AllocatorHandle;

pub mod array;
pub mod list;
pub mod primitive;
pub mod ptr;
pub mod reference;
pub mod texture;

pub mod prelude {
    pub use super::array::prelude::*;
    pub use super::list::prelude::*;
    pub use super::primitive::prelude::*;
    pub use super::ptr::prelude::*;
    pub use super::reference::prelude::*;
    pub use super::texture::prelude::*;
    pub use super::{SizeRefExt, SizeRefMutExt, SizedRefExt, SizedRefMutExt, SizedTypeExt, TypeExt};
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
#[repr(transparent)]
pub struct AllocationPointer {
    pub(crate) index: u64,
}

// pub trait InnerRef<'a>: Sized + Clone + Copy {
//     type OutputData: ?Sized;
//     type OutputType: TypeTrait;

//     unsafe fn from_raw_bytes(
//         bytes: &'a [u8],
//         ty: &'a Self::OutputType,
//         handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>;

//     unsafe fn raw_bytes(&self) -> &[u8];
//     unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()>;
// }

// pub trait InnerRefMut<'a>: Sized {
//     type OutputData: ?Sized;
//     type OutputType: TypeTrait;
//     type InnerRef: self::InnerRef<'a>;

//     unsafe fn from_raw_bytes(
//         bytes: &'a mut [u8],
//         ty: &'a Self::OutputType,
//         handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>;

//     unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()>;
// }

// pub trait InnerRefTypes<T: TypeTrait> {
//     type InnerRef<'a>: self::InnerRef<'a, OutputType = T>;
//     type InnerRefMut<'a>: self::InnerRefMut<'a, OutputType = T>;

//     fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a>;
// }

#[derive(Debug, Clone, Copy)]
pub enum Bytes<'a> {
    Bytes(&'a [u8]),
    Object { ty_name: &'static str, data: &'a dyn AllocatedType },
}

impl<'a> Bytes<'a> {
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

    pub fn downcast_ref_unwrap<T: Any>(self) -> &'a T {
        if TypeId::of::<T>() == TypeId::of::<[u8]>() {
            match self {
                Bytes::Bytes(inner) => unsafe { &*(inner as *const _ as *const T) },
                Bytes::Object { ty_name, .. } => {
                    panic!("Attempt to downcast type `{}` to `{}`.", ty_name, std::any::type_name::<T>())
                }
            }
        } else {
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
}

impl<'a> From<&'a [u8]> for Bytes<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Bytes::Bytes(bytes)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TypedBytes<'a> {
    bytes: Bytes<'a>,
    ty: &'a TypeEnum,
}

impl<'a> From<&'a AllocationInner> for TypedBytes<'a> {
    fn from(inner: &'a AllocationInner) -> Self {
        inner.as_ref()
    }
}

impl<'a> TypedBytes<'a> {
    pub fn from(bytes: impl Into<Bytes<'a>>, ty: &'a TypeEnum) -> Self {
        Self { bytes: bytes.into(), ty }
    }

    pub fn bytes(self) -> Bytes<'a> {
        self.bytes
    }

    pub fn ty(self) -> &'a TypeEnum {
        self.ty
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

    pub fn borrow<'b>(&'b self) -> Bytes<'b>
    where 'a: 'b {
        unsafe {
            match self {
                &BytesMut::Bytes(ref inner) => Bytes::Bytes(&*(*inner as *const _)),
                &BytesMut::Object { ty_name, ref data } => {
                    Bytes::Object { ty_name, data: &*(*data as *const _) }
                }
            }
        }
    }

    pub fn borrow_mut<'b>(&'b mut self) -> BytesMut<'b>
    where 'a: 'b {
        unsafe {
            match self {
                &mut BytesMut::Bytes(ref mut inner) => BytesMut::Bytes(&mut *(*inner as *mut _)),
                &mut BytesMut::Object { ty_name, ref mut data } => {
                    BytesMut::Object { ty_name, data: &mut *(*data as *mut _) }
                }
            }
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

    pub fn downcast_ref_unwrap<T: Any>(self) -> &'a T {
        if TypeId::of::<T>() == TypeId::of::<[u8]>() {
            match self {
                BytesMut::Bytes(inner) => unsafe { &*(inner as *const _ as *const T) },
                BytesMut::Object { ty_name, .. } => {
                    panic!("Attempt to downcast type `{}` to `{}`.", ty_name, std::any::type_name::<T>())
                }
            }
        } else {
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
    }

    pub fn downcast_mut_unwrap<T: Any>(self) -> &'a mut T {
        if TypeId::of::<T>() == TypeId::of::<[u8]>() {
            match self {
                BytesMut::Bytes(inner) => unsafe { &mut *(inner as *mut _ as *mut T) },
                BytesMut::Object { ty_name, .. } => {
                    panic!("Attempt to downcast type `{}` to `{}`.", ty_name, std::any::type_name::<T>())
                }
            }
        } else {
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
}

impl<'a> From<&'a mut [u8]> for BytesMut<'a> {
    fn from(bytes: &'a mut [u8]) -> Self {
        BytesMut::Bytes(bytes)
    }
}

#[derive(Debug)]
pub struct TypedBytesMut<'a> {
    bytes: BytesMut<'a>,
    ty: &'a TypeEnum,
}

impl<'a> From<&'a mut AllocationInner> for TypedBytesMut<'a> {
    fn from(inner: &'a mut AllocationInner) -> Self {
        inner.as_mut()
    }
}

impl<'a> TypedBytesMut<'a> {
    pub fn from(bytes: impl Into<BytesMut<'a>>, ty: &'a TypeEnum) -> Self {
        Self { bytes: bytes.into(), ty }
    }

    pub fn bytes_mut(self) -> BytesMut<'a> {
        self.bytes
    }

    pub fn bytes(self) -> Bytes<'a> {
        self.bytes.downgrade()
    }

    pub fn ty(self) -> &'a TypeEnum {
        self.ty
    }

    pub fn downgrade(self) -> TypedBytes<'a> {
        TypedBytes { bytes: self.bytes.downgrade(), ty: self.ty }
    }

    pub fn borrow<'b>(&'b self) -> TypedBytes<'b> {
        TypedBytes { bytes: self.bytes.borrow(), ty: unsafe { &*(self.ty as *const _) } }
    }

    pub fn borrow_mut<'b>(&'b mut self) -> TypedBytesMut<'b> {
        TypedBytesMut { bytes: self.bytes.borrow_mut(), ty: unsafe { &*(self.ty as *const _) } }
    }
}

impl<'a> From<TypedBytesMut<'a>> for (BytesMut<'a>, &'a TypeEnum) {
    fn from(bytes: TypedBytesMut<'a>) -> Self {
        (bytes.bytes, bytes.ty)
    }
}

pub struct DirectInnerRefTypes<T> {
    __marker: PhantomData<T>,
}

pub trait TypeExt: Into<TypeEnum> + PartialEq + Eq + Send + Sync + 'static {
    /// Returns the size of the associated value, in bytes.
    fn value_size_if_sized(&self) -> Option<usize>;

    /// Returns `true`, if the type can be safely cast/reinterpreted as another.
    /// Otherwise returns `false`.
    fn is_abi_compatible(&self, other: &Self) -> bool;

    /// Returns `true`, whether it is possible to let the user read the underlying
    /// binary representation of the associated value. Otherwise returns `false`.
    ///
    /// Pointers and unsized types are both a typical case which is not safe.
    fn has_safe_binary_representation(&self) -> bool;
}

pub trait SizedTypeExt: TypeExt {
    fn value_size(&self) -> usize;
}

pub trait TypeTrait: TypeExt + DowncastFromTypeEnum {}

pub trait DowncastFromTypeEnum {
    fn downcast_from_ref(from: &TypeEnum) -> Option<&Self>;
    fn downcast_from_mut(from: &mut TypeEnum) -> Option<&mut Self>;
}

pub trait InitializeWith<T>: Default {
    fn initialize_with(&mut self, descriptor: T);
}

impl InitializeWith<()> for () {
    fn initialize_with(&mut self, descriptor: ()) {}
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
}

// pub struct IndirectInnerRef<'a, T: AllocatedType> {
//     pub(crate) ptr: AllocationPointer,
//     pub(crate) ty: &'a T,
//     __marker: PhantomData<T>,
// }

// impl<'a, T: DynTypeTrait> IndirectInnerRef<'a, T> {
//     pub fn new(ptr: AllocationPointer) -> Self {
//         let (_, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };
//         let ty = ty.downcast_ref().unwrap();
//         Self { ptr, ty, __marker: Default::default() }
//     }
// }

// impl<'a, T: DynTypeTrait> Clone for IndirectInnerRef<'a, T> {
//     fn clone(&self) -> Self {
//         Self { ptr: self.ptr, ty: self.ty, __marker: Default::default() }
//     }
// }

// impl<'a, T: DynTypeTrait> Copy for IndirectInnerRef<'a, T> {}

// impl<'a, T: DynTypeTrait> InnerRef<'a> for IndirectInnerRef<'a, T> {
//     type OutputData = T::DynAlloc;
//     type OutputType = T;

//     unsafe fn from_raw_bytes(
//         bytes: &'a [u8],
//         ty: &'a Self::OutputType,
//         _handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>
//     {
//         if bytes.len() != std::mem::size_of::<AllocationPointer>() {
//             return Err(());
//         }
//         let mut read = Cursor::new(bytes);
//         let ptr = AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap());
//         Ok(Self { ptr, ty, __marker: Default::default() })
//     }

//     unsafe fn raw_bytes(&self) -> &[u8] {
//         safe_transmute::transmute_to_bytes(&[self.ptr.as_u64()])
//     }

//     unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()> {
//         let (data, ty) = Allocator::get().deref_ptr(self.ptr).unwrap();
//         let ty = ty.downcast_ref().ok_or(())?;

//         if ty != self.ty {
//             return Err(());
//         }

//         Ok((data.downcast_ref().ok_or(())?, ty))
//     }
// }

// pub struct IndirectInnerRefMut<'a, T: AllocatedType> {
//     pub(crate) ptr: AllocationPointer,
//     pub(crate) ty: &'a T,
//     __marker: PhantomData<T>,
// }

// impl<'a, T: DynTypeTrait> IndirectInnerRefMut<'a, T> {
//     pub fn new(ptr: AllocationPointer) -> Self {
//         let (_, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };
//         let ty = ty.downcast_ref().unwrap();
//         Self { ptr, ty, __marker: Default::default() }
//     }
// }

// impl<'a, T> InnerRefMut<'a> for IndirectInnerRefMut<'a, T>
// where T: DynTypeTrait
// {
//     type OutputData = T::DynAlloc;
//     type OutputType = T;
//     type InnerRef = IndirectInnerRef<'a, T>;

//     unsafe fn from_raw_bytes(
//         bytes: &'a mut [u8],
//         ty: &'a Self::OutputType,
//         _handle: AllocatorHandle<'a>,
//     ) -> Result<Self, ()>
//     {
//         if bytes.len() != std::mem::size_of::<AllocationPointer>() {
//             return Err(());
//         }
//         let mut read = Cursor::new(bytes);
//         let ptr = AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap());
//         Ok(Self { ptr, ty, __marker: Default::default() })
//     }

//     unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()> {
//         let (data, ty) = Allocator::get().deref_mut_ptr(self.ptr).unwrap();
//         let ty = ty.downcast_ref().ok_or(())?;

//         if ty != self.ty {
//             return Err(());
//         }

//         Ok((data.downcast_mut().ok_or(())?, ty))
//     }
// }

// pub struct IndirectInnerRefTypes<T> {
//     __marker: PhantomData<T>,
// }

// impl<T: DynTypeTrait> InnerRefTypes<T> for IndirectInnerRefTypes<T> {
//     type InnerRef<'a> = IndirectInnerRef<'a, T>;
//     type InnerRefMut<'a> = IndirectInnerRefMut<'a, T>;

//     fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a> {
//         IndirectInnerRef { ptr: from.ptr, ty: from.ty, __marker: Default::default() }
//     }
// }

impl<T> TypeTrait for T where T: DynTypeTrait {}

impl<T> TypeExt for T
where T: DynTypeTrait
{
    fn value_size_if_sized(&self) -> Option<usize> {
        None
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        true
    }

    fn has_safe_binary_representation(&self) -> bool {
        false
    }
}

pub trait SizedType: TypeTrait {}

macro_rules! define_type_enum {
    [
        $($variant:ident($inner:ident)),*$(,)?
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
        }
    }
}

define_type_enum![
    Shared(Shared),
    Unique(Unique),
    Primitive(PrimitiveType),
    // Tuple(Vec<Self>),
    Array(ArrayType),
    List(ListType),
    Texture(TextureType),
];

impl TypeEnum {
    pub fn downcast_ref<T: DowncastFromTypeEnum>(&self) -> Option<&T> {
        T::downcast_from_ref(self)
    }

    pub fn downcast_mut<T: DowncastFromTypeEnum>(&mut self) -> Option<&mut T> {
        T::downcast_from_mut(self)
    }
}

impl TypeExt for TypeEnum {
    fn value_size_if_sized(&self) -> Option<usize> {
        self.value_size_if_sized_impl()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        use TypeEnum::*;
        match (self, other) {
            (Primitive(a), Primitive(b)) => return a.is_abi_compatible(b),
            (Texture(a), Texture(b)) => return a.is_abi_compatible(b),
            _ => (),
        }
        if matches!(self, Array { .. }) || matches!(other, Array { .. }) {
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

            return a.is_abi_compatible(&b);
        }

        false
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
    R: RefMutExt<'a, T>,
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
    R: RefExt<'a, T>,
    T: TypeTrait + SizedTypeExt,
{
    fn value_size(&self) -> usize {
        let typed_bytes = unsafe { self.typed_bytes() };
        typed_bytes.ty().value_size_if_sized().unwrap()
    }

    fn bytes(&self) -> Option<&[u8]> {
        let typed_bytes = unsafe { self.typed_bytes() };

        if typed_bytes.ty().has_safe_binary_representation() {
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
where R: RefMutDynExt<'a>
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
where R: RefDynExt<'a>
{
    fn value_size_if_sized(&self) -> Option<usize> {
        let typed_bytes = unsafe { self.typed_bytes() };
        typed_bytes.ty().value_size_if_sized()
    }

    fn bytes_if_sized(&self) -> Option<&[u8]> {
        let typed_bytes = unsafe { self.typed_bytes() };
        let ty = typed_bytes.ty();

        if ty.value_size_if_sized().is_some() && ty.has_safe_binary_representation() {
            Some(typed_bytes.bytes().bytes().unwrap())
        } else {
            None
        }
    }
}
