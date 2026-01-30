#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{string::ToString, vec::Vec};

use bump_scope::{Bump, FixedBumpVec};

use common::TestWrap;

#[test]
fn owned_slice() {
    let bump: Bump = Bump::new();
    let slice = bump.alloc_iter((0..5).map(|v| v.to_string()).map(TestWrap));
    assert_eq!(
        slice.iter().map(|v| v.0.clone()).collect::<Vec<_>>(),
        &["0", "1", "2", "3", "4"]
    );

    for start in 0..slice.len() {
        for end in start..slice.len() {
            TestWrap::expect().drops(5).clones(5).run(|| {
                let mut slice_clone = bump.alloc_slice_clone(&slice);

                let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
                vec.append(slice_clone.drain(start..end));

                assert_eq!(vec, slice[start..end]);

                assert_eq!(
                    TestWrap::peel_slice(slice_clone.as_slice()),
                    TestWrap::peel_slice(&slice[..start])
                        .iter()
                        .chain(TestWrap::peel_slice(&slice[end..]))
                        .cloned()
                        .collect::<Vec<_>>()
                );
            });
        }
    }
}

#[test]
fn owned_slice_zst() {
    let bump: Bump = Bump::new();
    let slice = bump.alloc_iter((0..5).map(|v| v.to_string()).map(TestWrap));

    for start in 0..slice.len() {
        for end in start..slice.len() {
            TestWrap::expect().drops(5).clones(5).run(|| {
                let mut slice_clone = bump.alloc_slice_clone(&slice);
                let mut vec = FixedBumpVec::with_capacity_in(10, &bump);
                vec.append(slice_clone.drain(start..end));

                assert_eq!(vec.len(), end - start);
                assert_eq!(slice_clone.len(), slice[..start].len() + slice[end..].len());
            });
        }
    }
}
