use ::serde::Serialize;

use crate::{BumpBox, BumpString, BumpVec, FixedBumpString, FixedBumpVec, MutBumpString, MutBumpVec, MutBumpVecRev};

impl<T> Serialize for BumpBox<'_, T>
where
    T: Serialize + ?Sized,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(self, serializer)
    }
}

impl<T> Serialize for FixedBumpVec<'_, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <[T]>::serialize(self, serializer)
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool> Serialize for BumpVec<'_, '_, T, A, MIN_ALIGN, UP>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <[T]>::serialize(self, serializer)
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool> Serialize for MutBumpVec<'_, '_, T, A, MIN_ALIGN, UP>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <[T]>::serialize(self, serializer)
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool> Serialize for MutBumpVecRev<'_, '_, T, A, MIN_ALIGN, UP>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <[T]>::serialize(self, serializer)
    }
}

impl Serialize for FixedBumpString<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Serialize for BumpString<'_, '_, A, MIN_ALIGN, UP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool> Serialize for MutBumpString<'_, '_, A, MIN_ALIGN, UP> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}
