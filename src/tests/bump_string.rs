use super::either_way;
use crate::{Bump, BumpString, BumpVec};
use allocator_api2::alloc::Global;

either_way! {
    shrinks
    deallocates
    into_str
    into_str_without_shrink
}

fn shrinks<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut string = BumpString::from_str_in("1234", &bump);
    assert_eq!(bump.stats().allocated(), 4);
    string.pop();
    string.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 3);
    string.clear();
    string.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 0);
}

fn deallocates<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let string = BumpString::from_str_in("123", &bump);
    assert_eq!(bump.stats().allocated(), 3);
    drop(string);
    assert_eq!(bump.stats().allocated(), 0);
}

fn into_str<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut string = BumpString::from_str_in("12345", &bump);
    assert_eq!(bump.stats().allocated(), 5);
    string.truncate(3);
    let slice = string.into_str();
    assert_eq!(bump.stats().allocated(), 3);
    _ = slice;
}

fn into_str_without_shrink<const UP: bool>() {
    let bump: Bump<Global, 1, UP> = Bump::new();
    let mut string = BumpString::from_str_in("12345", &bump);
    assert_eq!(bump.stats().allocated(), 5);
    string.truncate(3);
    let slice = string.into_fixed_string().into_str();
    assert_eq!(bump.stats().allocated(), 5);
    _ = slice;
}
