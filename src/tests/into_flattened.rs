use std::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{Bump, BumpVec, FixedBumpVec, MutBumpVec, MutBumpVecRev};

fn items() -> impl Iterator<Item = [String; 3]> {
    // would use `array_chunks`, but it's not stable
    (1..)
        .map(|i| i.to_string())
        .take(3 * 3)
        .collect::<Vec<_>>()
        .chunks(3)
        .map(|s| s.to_vec().try_into().unwrap())
        .collect::<Vec<_>>()
        .into_iter()
}

#[test]
fn boxed() {
    let bump: Bump = Bump::new();
    let boxed = bump.alloc_iter(items());
    assert_eq!(format!("{boxed:?}"), r#"[["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"]]"#);
    let boxed = boxed.into_flattened();
    assert_eq!(format!("{boxed:?}"), r#"["1", "2", "3", "4", "5", "6", "7", "8", "9"]"#);
}

#[test]
fn fixed_vec() {
    let bump: Bump = Bump::new();
    let mut vec = FixedBumpVec::with_capacity_in(4, &bump);
    vec.extend(items());
    assert_eq!(format!("{vec:?}"), r#"[["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"]]"#);
    let vec = vec.into_flattened();
    assert_eq!(format!("{vec:?}"), r#"["1", "2", "3", "4", "5", "6", "7", "8", "9"]"#);
    assert_eq!(vec.capacity(), 12);
}

#[test]
fn vec() {
    let bump: Bump = Bump::new();
    let mut vec = BumpVec::with_capacity_in(4, &bump);
    vec.extend(items());
    assert_eq!(format!("{vec:?}"), r#"[["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"]]"#);
    let vec = vec.into_flattened();
    assert_eq!(format!("{vec:?}"), r#"["1", "2", "3", "4", "5", "6", "7", "8", "9"]"#);
    assert_eq!(vec.capacity(), 12);
}

#[test]
fn mut_vec() {
    let mut bump: Bump = Bump::new();
    let mut vec = MutBumpVec::with_capacity_in(4, &mut bump);
    let original_capacity = vec.capacity();
    vec.extend(items());
    assert_eq!(format!("{vec:?}"), r#"[["1", "2", "3"], ["4", "5", "6"], ["7", "8", "9"]]"#);
    let vec = vec.into_flattened();
    assert_eq!(format!("{vec:?}"), r#"["1", "2", "3", "4", "5", "6", "7", "8", "9"]"#);
    assert_eq!(vec.capacity(), original_capacity * 3);
}

#[test]
fn mut_vec_rev() {
    let mut bump: Bump = Bump::new();
    let mut vec = MutBumpVecRev::with_capacity_in(4, &mut bump);
    let original_capacity = vec.capacity();
    vec.extend(items());
    assert_eq!(format!("{vec:?}"), r#"[["7", "8", "9"], ["4", "5", "6"], ["1", "2", "3"]]"#);
    let vec = vec.into_flattened();
    assert_eq!(format!("{vec:?}"), r#"["7", "8", "9", "4", "5", "6", "1", "2", "3"]"#);
    assert_eq!(vec.capacity(), original_capacity * 3);
}
