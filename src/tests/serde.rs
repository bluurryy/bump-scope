use std::vec;

use ::serde::{de::DeserializeSeed, Serialize};

use crate::{bump_format, bump_vec, FixedBumpString, FixedBumpVec};

use super::*;

fn assert_same<A: Serialize, B: Serialize>(a: &A, b: &B) {
    let a_json = serde_json::to_string(a).unwrap();
    let b_json = serde_json::to_string(b).unwrap();
    assert_eq!(a_json, b_json);
}

#[test]
fn ser() {
    let mut bump: Bump = Bump::new();

    {
        let a = bump.alloc_str("Hello, world!");
        let b = "Hello, world!";
        assert_same(&a, &b);
    }

    {
        let mut a = FixedBumpVec::with_capacity_in(5, &bump);
        a.extend_from_slice_copy(&[1, 2, 3]);
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = bump_vec![in &bump; 1, 2, 3];
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = mut_bump_vec![in &mut bump; 1, 2, 3];
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = mut_bump_vec_rev![in &mut bump; 1, 2, 3];
        let b = vec![1, 2, 3];
        assert_same(&a, &b);
    }

    {
        let a = bump_format!(in &bump, "Hello, world!");
        let b = "Hello, world!";
        assert_same(&a, &b);
    }

    {
        let a = mut_bump_format!(in &mut bump, "Hello, world!");
        let b = "Hello, world!";
        assert_same(&a, &b);
    }
}

fn roundtrip<T>(src: &T, dst: &mut T)
where
    T: Serialize + PartialEq + Debug,
    for<'de, 'a> &'a mut T: DeserializeSeed<'de>,
{
    let json = serde_json::to_string(src).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json);
    dst.deserialize(&mut deserializer).unwrap();

    assert_eq!(*src, *dst);
}

#[test]
fn de() {
    let mut bump_src: Bump = Bump::new();
    let mut bump_dst: Bump = Bump::new();

    {
        let mut src = FixedBumpVec::with_capacity_in(3, &bump_src);
        src.extend_from_slice_copy(&[1, 2, 3]);
        let mut dst = FixedBumpVec::with_capacity_in(3, &bump_dst);
        roundtrip(&src, &mut dst);
    }

    {
        let src = bump_vec![in &bump_src; 1, 2, 3];
        let mut dst = bump_vec![in &bump_dst];
        roundtrip(&src, &mut dst);
    }

    {
        let src = mut_bump_vec![in &mut bump_src; 1, 2, 3];
        let mut dst = mut_bump_vec![in &mut bump_dst];
        roundtrip(&src, &mut dst);
    }

    {
        let src = mut_bump_vec_rev![in &mut bump_src; 1, 2, 3];
        let mut dst: MutBumpVecRev<i32, _> = mut_bump_vec_rev![in &mut bump_dst];

        let json = serde_json::to_string(&src).unwrap();
        let mut deserializer = serde_json::Deserializer::from_str(&json);
        dst.deserialize(&mut deserializer).unwrap();
        dst.reverse();

        assert_eq!(*src, *dst);
    }

    {
        let mut src = FixedBumpString::with_capacity_in(15, &bump_src);
        src.push_str("Hello, World!");
        let mut dst = FixedBumpString::with_capacity_in(15, &bump_dst);
        roundtrip(&src, &mut dst);
    }

    {
        let src = bump_format!(in &bump_src, "Hello, World!");
        let mut dst = bump_format!(in &bump_dst);
        roundtrip(&src, &mut dst);
    }

    {
        let src = mut_bump_format!(in &mut bump_src, "Hello, World!");
        let mut dst = mut_bump_format!(in &mut bump_dst);
        roundtrip(&src, &mut dst);
    }
}
