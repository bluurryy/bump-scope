use crate::{
    BaseAllocator, BumpAllocator, BumpBox, BumpString, BumpVec, FixedBumpString, FixedBumpVec, MinimumAlignment,
    MutBumpString, MutBumpVec, MutBumpVecRev, SupportedMinimumAlignment,
};
use ::serde::Serialize;
use allocator_api2::alloc::AllocError;
use core::fmt::Display;
use serde::{
    de::{self, DeserializeSeed, Expected, Visitor},
    Deserialize,
};

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

impl<T: Serialize, A: BumpAllocator> Serialize for BumpVec<T, A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <[T]>::serialize(self, serializer)
    }
}

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Serialize
    for MutBumpVec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Serialize
    for MutBumpVecRev<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<A: BumpAllocator> Serialize for BumpString<A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Serialize
    for MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}

const AN_ARRAY: &str = "an array";
const A_STRING: &str = "a string";

struct AllocationFailed;

impl Display for AllocationFailed {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("allocation failed")
    }
}

fn too_many_elements<E: de::Error>(len: usize, at_most: usize) -> E {
    E::invalid_length(len, &AtMost(at_most))
}

fn map_alloc_error<E: de::Error>(result: Result<(), AllocError>) -> Result<(), E> {
    match result {
        Ok(()) => Ok(()),
        Err(AllocError) => return Err(E::custom(&AllocationFailed)),
    }
}

struct AtMost(usize);

impl Expected for AtMost {
    fn fmt(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        write!(formatter, "at most {}", self.0)
    }
}

impl<'de, T> DeserializeSeed<'de> for &'_ mut FixedBumpVec<'_, T>
where
    T: Deserialize<'de>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T> Visitor<'de> for &'_ mut FixedBumpVec<'_, T>
where
    T: Deserialize<'de>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(AN_ARRAY)
    }

    fn visit_seq<Seq>(self, mut seq: Seq) -> Result<Self::Value, Seq::Error>
    where
        Seq: serde::de::SeqAccess<'de>,
    {
        let remaining = self.capacity() - self.len();

        if let Some(size_hint) = seq.size_hint() {
            if size_hint > remaining {
                return Err(too_many_elements(size_hint, remaining));
            }
        }

        while let Some(elem) = seq.next_element()? {
            if self.try_push(elem).is_err() {
                return Err(too_many_elements(self.len().saturating_add(1), remaining));
            }
        }

        Ok(())
    }
}

impl<'de, T: Deserialize<'de>, A: BumpAllocator> DeserializeSeed<'de> for &'_ mut BumpVec<T, A> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T: Deserialize<'de>, A: BumpAllocator> Visitor<'de> for &'_ mut BumpVec<T, A> {
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(AN_ARRAY)
    }

    fn visit_seq<Seq>(self, mut seq: Seq) -> Result<Self::Value, Seq::Error>
    where
        Seq: serde::de::SeqAccess<'de>,
    {
        if let Some(size_hint) = seq.size_hint() {
            map_alloc_error(self.try_reserve(size_hint))?;
        }

        while let Some(elem) = seq.next_element()? {
            map_alloc_error(self.try_push(elem))?;
        }

        Ok(())
    }
}

impl<'de, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DeserializeSeed<'de>
    for &'_ mut MutBumpVec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Visitor<'de>
    for &'_ mut MutBumpVec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(AN_ARRAY)
    }

    fn visit_seq<Seq>(self, mut seq: Seq) -> Result<Self::Value, Seq::Error>
    where
        Seq: serde::de::SeqAccess<'de>,
    {
        if let Some(size_hint) = seq.size_hint() {
            map_alloc_error(self.try_reserve(size_hint))?;
        }

        while let Some(elem) = seq.next_element()? {
            map_alloc_error(self.try_push(elem))?;
        }

        Ok(())
    }
}

impl<'de, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DeserializeSeed<'de>
    for &'_ mut MutBumpVecRev<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Visitor<'de>
    for &'_ mut MutBumpVecRev<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(AN_ARRAY)
    }

    fn visit_seq<Seq>(self, mut seq: Seq) -> Result<Self::Value, Seq::Error>
    where
        Seq: serde::de::SeqAccess<'de>,
    {
        if let Some(size_hint) = seq.size_hint() {
            map_alloc_error(self.try_reserve(size_hint))?;
        }

        while let Some(elem) = seq.next_element()? {
            map_alloc_error(self.try_push(elem))?;
        }

        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for &'_ mut FixedBumpString<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for &'_ mut FixedBumpString<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(A_STRING)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        map_alloc_error(self.try_push_str(v))
    }
}

impl<'de, A: BumpAllocator> DeserializeSeed<'de> for &'_ mut BumpString<A> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de, A: BumpAllocator> Visitor<'de> for &'_ mut BumpString<A> {
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(A_STRING)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        map_alloc_error(self.try_push_str(v))
    }
}

impl<'de, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DeserializeSeed<'de>
    for &'_ mut MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Visitor<'de>
    for &'_ mut MutBumpString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: BaseAllocator<GUARANTEED_ALLOCATED>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut alloc::fmt::Formatter) -> alloc::fmt::Result {
        formatter.write_str(A_STRING)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        map_alloc_error(self.try_push_str(v))
    }
}
