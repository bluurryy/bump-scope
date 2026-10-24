#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![expect(unused_allocation)]

use std::{
    alloc::Allocator,
    array,
    boxed::Box,
    string::{String, ToString},
    vec::{self, Vec},
};

use bump_scope::{
    BoxU, Bump, BumpAllocator as _, VecU,
    extra::{self, vec_from_owned_slice_in, vec_remove_allocator},
    owned_slice::{OwnedSlice, TakeOwnedSlice},
};

fn test_strings() -> [String; 3] {
    array::from_fn(|i| (i + 1).to_string())
}

fn test_strings_5() -> [String; 5] {
    array::from_fn(|i| i.to_string())
}

fn test_append<A: Allocator>(mut vec: Vec<String, A>, other: impl OwnedSlice<Item = String>) {
    extra::vec_append(&mut vec, other);
    assert_eq!(&*vec, ["1", "2", "3"]);
}

#[test]
fn append_vec() {
    let bump: Bump = Bump::new();
    let other_bump: Bump = Bump::new();

    {
        let vec = Vec::new_in(&bump);
        let mut other = test_strings().into_take_owned_slice();
        let other: &mut dyn TakeOwnedSlice<Item = String> = &mut other;
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: [String; 3] = test_strings();
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: Box<[String; 3]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: Box<[String]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: Vec<String> = Box::new(test_strings()).to_vec();
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: BoxU<[String; 3]> = other_bump.alloc(test_strings()).into_boxu();
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: BoxU<[String]> = other_bump.alloc(test_strings()).into_boxu();
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: VecU<String> = vec_remove_allocator(vec_from_owned_slice_in(test_strings(), &other_bump)).0;
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: Vec<String, _> = vec_from_owned_slice_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let other: vec::IntoIter<String> = Box::new(test_strings()).to_vec().into_iter();
        test_append(vec, other);
    }

    {
        let vec = Vec::new_in(&bump);
        let mut other: Vec<String> = Box::new(test_strings_5()).to_vec();
        let other: vec::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }
}
