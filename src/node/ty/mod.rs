use std::any::TypeId;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::io::{Cursor, Read, Write};
use std::marker::PhantomData;

use byteorder::{ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

pub use array::*;
use downcast_rs::Downcast;
pub use list::*;
pub use primitive::*;
pub use reference::*;
pub use texture::*;

use crate::graph::alloc::{AllocatedType, Allocator};

use super::behaviour::AllocatorHandle;

pub mod array;
pub mod list;
pub mod primitive;
pub mod reference;
pub mod texture;

pub mod prelude {
    pub use super::reference::{RefDynExt, RefExt, RefMutDynExt, RefMutExt};
    pub use super::{InnerRef, InnerRefMut};
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Hash)]
#[repr(transparent)]
pub struct AllocationPointer {
    pub(crate) index: u64,
}

pub trait InnerRef<'a>: Sized + Clone + Copy {
    type OutputData: ?Sized;
    type OutputType: TypeTrait;

    unsafe fn from_raw_bytes(
        bytes: &'a [u8],
        ty: &'a Self::OutputType,
        handle: AllocatorHandle<'a>,
    ) -> Result<Self, ()>;

    unsafe fn raw_bytes(&self) -> &[u8];
    unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()>;
}

pub trait InnerRefMut<'a>: Sized {
    type OutputData: ?Sized;
    type OutputType: TypeTrait;
    type InnerRef: self::InnerRef<'a>;

    unsafe fn from_raw_bytes(
        bytes: &'a mut [u8],
        ty: &'a Self::OutputType,
        handle: AllocatorHandle<'a>,
    ) -> Result<Self, ()>;

    unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()>;
}

pub trait InnerRefTypes<T: TypeTrait> {
    type InnerRef<'a>: self::InnerRef<'a, OutputType = T>;
    type InnerRefMut<'a>: self::InnerRefMut<'a, OutputType = T>;

    fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a>;
}

pub struct DirectInnerRef<'a, T: TypeTrait> {
    bytes: &'a [u8],
    ty: &'a T,
}

impl<'a, T: TypeTrait> Clone for DirectInnerRef<'a, T> {
    fn clone(&self) -> Self {
        Self { bytes: self.bytes, ty: self.ty }
    }
}

impl<'a, T: TypeTrait> Copy for DirectInnerRef<'a, T> {}

impl<'a, T: TypeTrait> InnerRef<'a> for DirectInnerRef<'a, T> {
    type OutputData = [u8];
    type OutputType = T;

    unsafe fn from_raw_bytes(
        bytes: &'a [u8],
        ty: &'a Self::OutputType,
        _handle: AllocatorHandle<'a>,
    ) -> Result<Self, ()>
    {
        Ok(Self { bytes, ty })
    }

    unsafe fn raw_bytes(&self) -> &[u8] {
        &self.bytes
    }

    unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()> {
        Ok((self.bytes, self.ty))
    }
}

pub struct DirectInnerRefMut<'a, T: TypeTrait> {
    bytes: &'a mut [u8],
    ty: &'a T,
}

impl<'a, T: TypeTrait> InnerRefMut<'a> for DirectInnerRefMut<'a, T> {
    type OutputData = [u8];
    type OutputType = T;
    type InnerRef = DirectInnerRef<'a, T>;

    unsafe fn from_raw_bytes(
        bytes: &'a mut [u8],
        ty: &'a Self::OutputType,
        _handle: AllocatorHandle<'a>,
    ) -> Result<Self, ()>
    {
        Ok(Self { bytes, ty })
    }

    unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()> {
        Ok((self.bytes, self.ty))
    }
}

pub struct DirectInnerRefTypes<T> {
    __marker: PhantomData<T>,
}

impl<T: TypeTrait> InnerRefTypes<T> for DirectInnerRefTypes<T> {
    type InnerRef<'a> = DirectInnerRef<'a, T>;
    type InnerRefMut<'a> = DirectInnerRefMut<'a, T>;

    fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a> {
        DirectInnerRef { bytes: &*from.bytes, ty: from.ty }
    }
}

pub trait TypeExt: Into<TypeEnum> + PartialEq + Eq + Send + Sync + 'static {
    /// Returns the size of the associated value, in bytes.
    fn value_size(&self) -> usize;

    /// Returns `true`, if the type can be safely cast/reinterpreted as another.
    /// Otherwise returns `false`.
    fn is_abi_compatible(&self, other: &Self) -> bool;

    /// Returns `true`, whether it is possible to let the user read the underlying
    /// binary representation of the associated value. Otherwise returns `false`.
    ///
    /// Pointers are the typical case which is not safe.
    fn has_safe_binary_representation(&self) -> bool;
}

pub trait TypeTrait: TypeExt + DowncastFromTypeEnum {
    type InnerRefTypes: self::InnerRefTypes<Self> = DirectInnerRefTypes<Self>;
}

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
where Self: TypeTrait<InnerRefTypes = IndirectInnerRefTypes<Self>>
{
    /// The type to initialize the allocation with.
    type Descriptor: DynTypeDescriptor<Self>;
    type DynAlloc: AllocatedType;

    fn create_value_from_descriptor(descriptor: Self::Descriptor) -> Self::DynAlloc;
}

pub struct IndirectInnerRef<'a, T: AllocatedType> {
    pub(crate) ptr: AllocationPointer,
    pub(crate) ty: &'a T,
    __marker: PhantomData<T>,
}

impl<'a, T: DynTypeTrait> IndirectInnerRef<'a, T> {
    pub fn new(ptr: AllocationPointer) -> Self {
        let (_, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };
        let ty = ty.downcast_ref().unwrap();
        Self { ptr, ty, __marker: Default::default() }
    }
}

impl<'a, T: DynTypeTrait> Clone for IndirectInnerRef<'a, T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr, ty: self.ty, __marker: Default::default() }
    }
}

impl<'a, T: DynTypeTrait> Copy for IndirectInnerRef<'a, T> {}

impl<'a, T: DynTypeTrait> InnerRef<'a> for IndirectInnerRef<'a, T> {
    type OutputData = T::DynAlloc;
    type OutputType = T;

    unsafe fn from_raw_bytes(
        bytes: &'a [u8],
        ty: &'a Self::OutputType,
        _handle: AllocatorHandle<'a>,
    ) -> Result<Self, ()>
    {
        if bytes.len() != std::mem::size_of::<AllocationPointer>() {
            return Err(());
        }
        let mut read = Cursor::new(bytes);
        let ptr = AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap());
        Ok(Self { ptr, ty, __marker: Default::default() })
    }

    unsafe fn raw_bytes(&self) -> &[u8] {
        safe_transmute::transmute_to_bytes(&[self.ptr.as_u64()])
    }

    unsafe fn deref_ref(self) -> Result<(&'a Self::OutputData, &'a Self::OutputType), ()> {
        let (data, ty) = Allocator::get().deref_ptr(self.ptr).unwrap();
        let ty = ty.downcast_ref().ok_or(())?;

        if ty != self.ty {
            return Err(());
        }

        Ok((data.downcast_ref().ok_or(())?, ty))
    }
}

pub struct IndirectInnerRefMut<'a, T: AllocatedType> {
    pub(crate) ptr: AllocationPointer,
    pub(crate) ty: &'a T,
    __marker: PhantomData<T>,
}

impl<'a, T: DynTypeTrait> IndirectInnerRefMut<'a, T> {
    pub fn new(ptr: AllocationPointer) -> Self {
        let (_, ty) = unsafe { Allocator::get().deref_ptr(ptr).unwrap() };
        let ty = ty.downcast_ref().unwrap();
        Self { ptr, ty, __marker: Default::default() }
    }
}

impl<'a, T> InnerRefMut<'a> for IndirectInnerRefMut<'a, T>
where T: DynTypeTrait
{
    type OutputData = T::DynAlloc;
    type OutputType = T;
    type InnerRef = IndirectInnerRef<'a, T>;

    unsafe fn from_raw_bytes(
        bytes: &'a mut [u8],
        ty: &'a Self::OutputType,
        _handle: AllocatorHandle<'a>,
    ) -> Result<Self, ()>
    {
        if bytes.len() != std::mem::size_of::<AllocationPointer>() {
            return Err(());
        }
        let mut read = Cursor::new(bytes);
        let ptr = AllocationPointer::new(read.read_u64::<LittleEndian>().unwrap());
        Ok(Self { ptr, ty, __marker: Default::default() })
    }

    unsafe fn deref_ref_mut(self) -> Result<(&'a mut Self::OutputData, &'a Self::OutputType), ()> {
        let (data, ty) = Allocator::get().deref_mut_ptr(self.ptr).unwrap();
        let ty = ty.downcast_ref().ok_or(())?;

        if ty != self.ty {
            return Err(());
        }

        Ok((data.downcast_mut().ok_or(())?, ty))
    }
}

pub struct IndirectInnerRefTypes<T> {
    __marker: PhantomData<T>,
}

impl<T: DynTypeTrait> InnerRefTypes<T> for IndirectInnerRefTypes<T> {
    type InnerRef<'a> = IndirectInnerRef<'a, T>;
    type InnerRefMut<'a> = IndirectInnerRefMut<'a, T>;

    fn downgrade<'a>(from: Self::InnerRefMut<'a>) -> Self::InnerRef<'a> {
        IndirectInnerRef { ptr: from.ptr, ty: from.ty, __marker: Default::default() }
    }
}

impl<T> TypeTrait for T
where T: DynTypeTrait
{
    type InnerRefTypes = IndirectInnerRefTypes<Self>;
}

impl<T> TypeExt for T
where T: DynTypeTrait
{
    fn value_size(&self) -> usize {
        std::mem::size_of::<AllocationPointer>()
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        true
    }

    fn has_safe_binary_representation(&self) -> bool {
        false
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub enum TypeEnum {
    Primitive(PrimitiveType),
    // Tuple(Vec<Self>),
    Array(ArrayType),
    List(ListType),
    Texture(TextureType),
}

impl TypeEnum {
    pub fn downcast_ref<T: DowncastFromTypeEnum>(&self) -> Option<&T> {
        T::downcast_from_ref(self)
    }

    pub fn downcast_mut<T: DowncastFromTypeEnum>(&mut self) -> Option<&mut T> {
        T::downcast_from_mut(self)
    }
}

impl Display for TypeEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use TypeEnum::*;
        match self {
            Primitive(primitive) => f.write_fmt(format_args!("{}", primitive)),
            Array(array) => f.write_fmt(format_args!("{}", array)),
            List(list) => f.write_fmt(format_args!("{}", list)),
            Texture(texture) => f.write_fmt(format_args!("{}", texture)),
        }
    }
}

impl TypeExt for TypeEnum {
    fn value_size(&self) -> usize {
        use TypeEnum::*;
        match self {
            Primitive(primitive) => primitive.value_size(),
            Array(array) => array.value_size(),
            List(list) => list.value_size(),
            Texture(texture) => texture.value_size(),
        }
    }

    fn is_abi_compatible(&self, other: &Self) -> bool {
        use TypeEnum::*;
        match (self, other) {
            (Primitive(a), Primitive(b)) => return a.is_abi_compatible(b),
            (Texture(a), Texture(b)) => return a.is_abi_compatible(b),
            _ => (),
        }
        if matches!(self, Array { .. }) || matches!(other, Array { .. }) {
            let a = if let Array(array) = self {
                Cow::Borrowed(array)
            } else {
                Cow::Owned(ArrayType::single(self.clone()))
            };
            let b = if let Array(array) = other {
                Cow::Borrowed(array)
            } else {
                Cow::Owned(ArrayType::single(other.clone()))
            };
            return a.is_abi_compatible(&b);
        }

        false
    }

    fn has_safe_binary_representation(&self) -> bool {
        use TypeEnum::*;
        match self {
            Primitive(primitive) => primitive.has_safe_binary_representation(),
            Array(array) => array.has_safe_binary_representation(),
            List(list) => list.has_safe_binary_representation(),
            Texture(texture) => texture.has_safe_binary_representation(),
        }
    }
}
