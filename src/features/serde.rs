use core::fmt::Display;

use ::serde::Serialize;
use allocator_api2::alloc::{AllocError, Allocator};
use serde::{
    de::{self, DeserializeSeed, Expected, Visitor},
    Deserialize,
};

use crate::{
    Box, FixedString, FixedVec, MinimumAlignment, MutString, MutVec, MutVecRev, String, SupportedMinimumAlignment, Vec,
};

impl<T> Serialize for Box<'_, T>
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

impl<T> Serialize for FixedVec<'_, T>
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
    for Vec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
    for MutVec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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
    for MutVecRev<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl Serialize for FixedString<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Serialize
    for String<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        str::serialize(self, serializer)
    }
}

impl<A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> Serialize
    for MutString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
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

impl<'de, T> DeserializeSeed<'de> for &'_ mut FixedVec<'_, T>
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

impl<'de, T> Visitor<'de> for &'_ mut FixedVec<'_, T>
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

impl<'de, T, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DeserializeSeed<'de>
    for &'_ mut Vec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut Vec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut MutVec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut MutVec<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut MutVecRev<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut MutVecRev<'_, '_, T, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    T: Deserialize<'de>,
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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

impl<'de> DeserializeSeed<'de> for &'_ mut FixedString<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for &'_ mut FixedString<'_> {
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
    for &'_ mut String<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut String<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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

impl<'de, A, const MIN_ALIGN: usize, const UP: bool, const GUARANTEED_ALLOCATED: bool> DeserializeSeed<'de>
    for &'_ mut MutString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
    for &'_ mut MutString<'_, '_, A, MIN_ALIGN, UP, GUARANTEED_ALLOCATED>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    A: Allocator + Clone,
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
