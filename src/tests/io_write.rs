#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::{
    io::{ErrorKind, IoSlice, Write},
    vec::Vec,
};

use crate::{
    alloc::{Allocator, Global},
    Bump, BumpVec, FixedBumpVec, MutBumpVec,
};

use super::limited_allocator::Limited;

#[test]
fn io_write_vec() {
    let bump: Bump<Limited<Global>> = Bump::new_in(Limited::new_in(512, Global));
    let mut vec = BumpVec::new_in(&bump);
    assert!(matches!(vec.write(&[1, 2, 3]), Ok(3)));
    assert!(matches!(
        vec.write_vectored(&[IoSlice::new(&[4, 5]), IoSlice::new(&[6, 7, 8]),]),
        Ok(5)
    ));
    vec.write_all(&[9]).unwrap();
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let too_much = (0..1024).map(|i| i as u8).collect::<Vec<_>>();
    assert_eq!(vec.write(&too_much).unwrap_err().kind(), ErrorKind::OutOfMemory);
    assert_eq!(vec.write_all(&too_much).unwrap_err().kind(), ErrorKind::OutOfMemory);
    assert_eq!(
        vec.write_vectored(&[IoSlice::new(&[10]), IoSlice::new(&too_much)])
            .unwrap_err()
            .kind(),
        ErrorKind::OutOfMemory
    );
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn io_write_mut_vec() {
    let mut bump: Bump<Limited<Global>> = Bump::new_in(Limited::new_in(512, Global));
    let mut vec = MutBumpVec::new_in(&mut bump);
    assert!(matches!(vec.write(&[1, 2, 3]), Ok(3)));
    assert!(matches!(
        vec.write_vectored(&[IoSlice::new(&[4, 5]), IoSlice::new(&[6, 7, 8]),]),
        Ok(5)
    ));
    vec.write_all(&[9]).unwrap();
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let too_much = (0..1024).map(|i| i as u8).collect::<Vec<_>>();
    assert_eq!(vec.write(&too_much).unwrap_err().kind(), ErrorKind::OutOfMemory);
    assert_eq!(vec.write_all(&too_much).unwrap_err().kind(), ErrorKind::OutOfMemory);
    assert_eq!(
        vec.write_vectored(&[IoSlice::new(&[10]), IoSlice::new(&too_much)])
            .unwrap_err()
            .kind(),
        ErrorKind::OutOfMemory
    );
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn io_write_fixed_vec() {
    let bump: Bump = Bump::new();
    let mut vec = FixedBumpVec::with_capacity_in(1024, &bump);
    assert!(matches!(vec.write(&[1, 2, 3]), Ok(3)));
    assert!(matches!(
        vec.write_vectored(&[IoSlice::new(&[4, 5]), IoSlice::new(&[6, 7, 8]),]),
        Ok(5)
    ));
    vec.write_all(&[9]).unwrap();
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let too_much = (0..1024).map(|i| i as u8).collect::<Vec<_>>();
    assert_eq!(vec.write(&too_much).unwrap_err().kind(), ErrorKind::OutOfMemory);
    assert_eq!(vec.write_all(&too_much).unwrap_err().kind(), ErrorKind::OutOfMemory);
    assert_eq!(
        vec.write_vectored(&[IoSlice::new(&[10]), IoSlice::new(&too_much)])
            .unwrap_err()
            .kind(),
        ErrorKind::OutOfMemory
    );
    assert_eq!(vec, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}
