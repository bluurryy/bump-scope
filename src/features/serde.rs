use core::fmt::{self, Display};

use serde::{
    de::{self, DeserializeSeed, Expected, Visitor},
    Deserialize, Serialize,
};

use crate::{
    alloc_reexport::alloc::AllocError, BumpAllocator, BumpBox, BumpString, BumpVec, FixedBumpString, FixedBumpVec,
    MutBumpAllocator, MutBumpString, MutBumpVec, MutBumpVecRev,
};

impl<T: Serialize + ?Sized> Serialize for BumpBox<'_, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(self, serializer)
    }
}

impl<T: Serialize> Serialize for FixedBumpVec<'_, T> {
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

impl<T: Serialize, A> Serialize for MutBumpVec<T, A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        <[T]>::serialize(self, serializer)
    }
}

impl<T: Serialize, A> Serialize for MutBumpVecRev<T, A> {
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

impl<A> Serialize for MutBumpString<A> {
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("allocation failed")
    }
}

fn too_many_elements<E: de::Error>(len: usize, at_most: usize) -> E {
    E::invalid_length(len, &AtMost(at_most))
}

fn map_alloc_error<E: de::Error>(result: Result<(), AllocError>) -> Result<(), E> {
    match result {
        Ok(()) => Ok(()),
        Err(AllocError) => Err(E::custom(&AllocationFailed)),
    }
}

struct AtMost(usize);

impl Expected for AtMost {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "at most {}", self.0)
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for &'_ mut FixedBumpVec<'_, T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for &'_ mut FixedBumpVec<'_, T> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

impl<'de, T: Deserialize<'de>, A: MutBumpAllocator> DeserializeSeed<'de> for &'_ mut MutBumpVec<T, A> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T: Deserialize<'de>, A: MutBumpAllocator> Visitor<'de> for &'_ mut MutBumpVec<T, A> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

impl<'de, T: Deserialize<'de>, A: MutBumpAllocator> DeserializeSeed<'de> for &mut MutBumpVecRev<T, A> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'de, T: Deserialize<'de>, A: MutBumpAllocator> Visitor<'de> for &mut MutBumpVecRev<T, A> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

impl Visitor<'_> for &'_ mut FixedBumpString<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

impl<A: BumpAllocator> Visitor<'_> for &'_ mut BumpString<A> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(A_STRING)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        map_alloc_error(self.try_push_str(v))
    }
}

impl<'de, A: MutBumpAllocator> DeserializeSeed<'de> for &'_ mut MutBumpString<A> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(self)
    }
}

impl<A: MutBumpAllocator> Visitor<'_> for &mut MutBumpString<A> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(A_STRING)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        map_alloc_error(self.try_push_str(v))
    }
}
