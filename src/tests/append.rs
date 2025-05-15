#![allow(unused_allocation, clippy::unnecessary_to_owned)]

use std::{
    array,
    boxed::Box,
    ops::Deref,
    string,
    string::{String, ToString},
    vec::{self, Vec},
};

use crate::{
    owned_slice::{self, OwnedSlice, TakeOwnedSlice},
    Bump, BumpAllocator, BumpBox, BumpVec, FixedBumpVec, MutBumpAllocator, MutBumpVec, MutBumpVecRev,
};

trait Append<T>: Deref<Target = [T]> {
    fn append(&mut self, other: impl OwnedSlice<Item = T>);
}

impl<T> Append<T> for FixedBumpVec<'_, T> {
    fn append(&mut self, other: impl OwnedSlice<Item = T>) {
        FixedBumpVec::append(self, other);
    }
}

impl<T, A: BumpAllocator> Append<T> for BumpVec<T, A> {
    fn append(&mut self, other: impl OwnedSlice<Item = T>) {
        BumpVec::append(self, other);
    }
}

impl<T, A: MutBumpAllocator> Append<T> for MutBumpVec<T, A> {
    fn append(&mut self, other: impl OwnedSlice<Item = T>) {
        MutBumpVec::append(self, other);
    }
}

impl<T, A: MutBumpAllocator> Append<T> for MutBumpVecRev<T, A> {
    fn append(&mut self, other: impl OwnedSlice<Item = T>) {
        MutBumpVecRev::append(self, other);
    }
}

fn test_strings() -> [String; 3] {
    array::from_fn(|i| (i + 1).to_string())
}

fn test_strings_5() -> [String; 5] {
    array::from_fn(|i| i.to_string())
}

fn test_append(mut vec: impl Append<String>, other: impl OwnedSlice<Item = String>) {
    vec.append(other);
    assert_eq!(&*vec, ["1", "2", "3"]);
}

#[test]
fn append_fixed_vec() {
    let bump: Bump = Bump::new();
    let mut other_bump: Bump = Bump::new();

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let mut other = test_strings().into_take_owned_slice();
        let other: &mut dyn TakeOwnedSlice<Item = String> = &mut other;
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: [String; 3] = test_strings();
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: Box<[String; 3]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: Box<[String]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: Vec<String> = Box::new(test_strings()).to_vec();
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: BumpBox<[String; 3]> = other_bump.alloc(test_strings());
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: BumpBox<[String]> = other_bump.alloc(test_strings()).into_unsized();
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: FixedBumpVec<String> = FixedBumpVec::from_iter_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: BumpVec<String, _> = BumpVec::from_array_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: MutBumpVec<String, _> = MutBumpVec::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: MutBumpVecRev<String, _> = MutBumpVecRev::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: owned_slice::IntoIter<String> = other_bump.alloc(test_strings()).into_unsized().into_iter();
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let mut other: BumpBox<[String]> = other_bump.alloc(test_strings_5()).into_unsized();
        let other: owned_slice::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let other: vec::IntoIter<String> = Box::new(test_strings()).to_vec().into_iter();
        test_append(vec, other);
    }

    {
        let vec = FixedBumpVec::with_capacity_in(10, &bump);
        let mut other: Vec<String> = Box::new(test_strings_5()).to_vec();
        let other: vec::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }
}

#[test]
fn append_vec() {
    let bump: Bump = Bump::new();
    let mut other_bump: Bump = Bump::new();

    {
        let vec = BumpVec::new_in(&bump);
        let mut other = test_strings().into_take_owned_slice();
        let other: &mut dyn TakeOwnedSlice<Item = String> = &mut other;
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: [String; 3] = test_strings();
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: Box<[String; 3]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: Box<[String]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: Vec<String> = Box::new(test_strings()).to_vec();
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: BumpBox<[String; 3]> = other_bump.alloc(test_strings());
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: BumpBox<[String]> = other_bump.alloc(test_strings()).into_unsized();
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: FixedBumpVec<String> = FixedBumpVec::from_iter_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: BumpVec<String, _> = BumpVec::from_array_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: MutBumpVec<String, _> = MutBumpVec::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: MutBumpVecRev<String, _> = MutBumpVecRev::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: owned_slice::IntoIter<String> = other_bump.alloc(test_strings()).into_unsized().into_iter();
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let mut other: BumpBox<[String]> = other_bump.alloc(test_strings_5()).into_unsized();
        let other: owned_slice::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let other: vec::IntoIter<String> = Box::new(test_strings()).to_vec().into_iter();
        test_append(vec, other);
    }

    {
        let vec = BumpVec::new_in(&bump);
        let mut other: Vec<String> = Box::new(test_strings_5()).to_vec();
        let other: vec::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }
}

#[test]
fn append_mut_vec() {
    let mut bump: Bump = Bump::new();
    let mut other_bump: Bump = Bump::new();

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let mut other = test_strings().into_take_owned_slice();
        let other: &mut dyn TakeOwnedSlice<Item = String> = &mut other;
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: [String; 3] = test_strings();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: Box<[String; 3]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: Box<[String]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: Vec<String> = Box::new(test_strings()).to_vec();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: BumpBox<[String; 3]> = other_bump.alloc(test_strings());
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: BumpBox<[String]> = other_bump.alloc(test_strings()).into_unsized();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: FixedBumpVec<String> = FixedBumpVec::from_iter_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: BumpVec<String, _> = BumpVec::from_array_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: MutBumpVec<String, _> = MutBumpVec::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: MutBumpVecRev<String, _> = MutBumpVecRev::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: owned_slice::IntoIter<String> = other_bump.alloc(test_strings()).into_unsized().into_iter();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let mut other: BumpBox<[String]> = other_bump.alloc(test_strings_5()).into_unsized();
        let other: owned_slice::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let other: vec::IntoIter<String> = Box::new(test_strings()).to_vec().into_iter();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVec::new_in(&mut bump);
        let mut other: Vec<String> = Box::new(test_strings_5()).to_vec();
        let other: vec::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }
}

#[test]
fn append_mut_vec_rev() {
    let mut bump: Bump = Bump::new();
    let mut other_bump: Bump = Bump::new();

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let mut other = test_strings().into_take_owned_slice();
        let other: &mut dyn TakeOwnedSlice<Item = String> = &mut other;
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: [String; 3] = test_strings();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: Box<[String; 3]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: Box<[String]> = Box::new(test_strings());
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: Vec<String> = Box::new(test_strings()).to_vec();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: BumpBox<[String; 3]> = other_bump.alloc(test_strings());
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: BumpBox<[String]> = other_bump.alloc(test_strings()).into_unsized();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: FixedBumpVec<String> = FixedBumpVec::from_iter_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: BumpVec<String, _> = BumpVec::from_array_in(test_strings(), &other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: MutBumpVecRev<String, _> = MutBumpVecRev::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: MutBumpVecRev<String, _> = MutBumpVecRev::from_array_in(test_strings(), &mut other_bump);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: owned_slice::IntoIter<String> = other_bump.alloc(test_strings()).into_unsized().into_iter();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let mut other: BumpBox<[String]> = other_bump.alloc(test_strings_5()).into_unsized();
        let other: owned_slice::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let other: vec::IntoIter<String> = Box::new(test_strings()).to_vec().into_iter();
        test_append(vec, other);
    }

    {
        let vec = MutBumpVecRev::new_in(&mut bump);
        let mut other: Vec<String> = Box::new(test_strings_5()).to_vec();
        let other: vec::Drain<String> = other.drain(1..4);
        test_append(vec, other);
    }
}
