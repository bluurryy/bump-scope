use std::{panic::catch_unwind, string::String};

use crate::{Bump, BumpBox};

#[test]
fn fn_once() {
    let bump: Bump = Bump::new();

    let x: BumpBox<dyn FnOnce()> = bump.alloc(|| {});
    (x)();

    // making sure the string is dropped
    let string = String::from("hello world");
    let x: BumpBox<dyn FnOnce()> = bump.alloc(move || drop(string));
    (x)();

    let string = String::from("hello world");
    let x: BumpBox<dyn FnOnce()> = bump.alloc(move || drop(string));
    drop(x);

    #[allow(unused_variables)]
    catch_unwind(|| {
        let string = String::from("hello world");
        let x: BumpBox<dyn FnOnce()> = bump.alloc(move || {
            panic!("a");
            #[allow(unreachable_code)]
            drop(string);
        });
        (x)();
    })
    .unwrap_err();
}

#[test]
fn does_impl() {
    fn impls_fn_once<T: FnOnce()>(_: &T) {}
    fn impls_fn_mut<T: FnMut()>(_: &T) {}
    fn impls_fn<T: Fn()>(_: &T) {}

    let bump: Bump = Bump::new();

    let f = bump.alloc(|| {});
    impls_fn_once(&f);
    impls_fn_mut(&f);
    impls_fn(&f);

    let mut string = String::from("hello");
    let f = bump.alloc(|| string.push('x'));
    impls_fn_once(&f);
    impls_fn_mut(&f);

    let string = String::from("hello");
    let f = bump.alloc(|| drop(string));
    impls_fn_once(&f);

    let f: BumpBox<dyn Fn()> = bump.alloc(|| {});
    impls_fn_once(&f);
    impls_fn_mut(&f);
    impls_fn(&f);

    let mut string = String::from("hello");
    let f: BumpBox<dyn FnMut()> = bump.alloc(|| string.push('x'));
    impls_fn_once(&f);
    impls_fn_mut(&f);

    let string = String::from("hello");
    let f: BumpBox<dyn FnOnce()> = bump.alloc(|| drop(string));
    impls_fn_once(&f);
}
