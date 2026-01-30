#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![cfg(feature = "nightly-clone-to-uninit")]
#![feature(clone_to_uninit, ptr_metadata)]

use std::{
    clone::CloneToUninit,
    fmt::Display,
    format,
    string::{String, ToString as _},
    vec::Vec,
};

use bump_scope::Bump;

#[test]
fn test_slice() {
    let strings = (1..=3).map(|i| i.to_string()).collect::<Vec<_>>();
    let slice: &[String] = &strings;

    let bump: Bump = Bump::new();
    let allocated = bump.alloc_clone(slice);
    assert_eq!(&*allocated, ["1", "2", "3"]);
}

#[test]
fn test_trait_object() {
    trait DisplayClone: Display + CloneToUninit {}
    impl<T: ?Sized + Display + CloneToUninit> DisplayClone for T {}

    let string = String::from("Hello, world!");
    let object: &dyn DisplayClone = &string;

    let bump: Bump = Bump::new();
    let allocated = bump.alloc_clone(object);
    assert_eq!(allocated.to_string(), "Hello, world!");
}

#[test]
fn test_trait_object_fn() {
    trait FnClone: Fn() -> String + CloneToUninit {}
    impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}

    let reference = &String::from("Hello,");
    let value = String::from("world!");

    let closure = move || format!("{reference} {value}");
    let object: &dyn FnClone = &closure;

    assert_eq!(object(), "Hello, world!");

    let bump: Bump = Bump::new();
    let object_clone = bump.alloc_clone(object);

    assert_eq!(object_clone(), "Hello, world!");
}
