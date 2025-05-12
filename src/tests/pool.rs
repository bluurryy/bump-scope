use std::vec::Vec;

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{alloc::Global, BumpPool};

use super::either_way;

either_way! {
    rayon

    scope
}

fn rayon<const UP: bool>() {
    if cfg!(miri) {
        // rayon violates strict-provenance :(
        return;
    }

    let mut pool = BumpPool::<Global, 1, UP>::new();

    let ints: Vec<&mut usize> = (0..1000usize)
        .into_par_iter()
        .map_init(
            || pool.get(),
            |bump, i| {
                // do some expensive work
                bump.alloc(i).into_mut()
            },
        )
        .collect();

    assert!((0..1000usize).eq(ints.iter().map(|i| **i)));

    pool.reset();
}

fn scope<const UP: bool>() {
    let pool = BumpPool::<Global, 1, UP>::new();
    let (sender, receiver) = std::sync::mpsc::sync_channel(10);

    std::thread::scope(|s| {
        s.spawn(|| {
            let bump = pool.get();
            let string = bump.alloc_str("Hello").into_ref();
            sender.send(string).unwrap();
            drop(sender);
        });

        s.spawn(|| {
            for string in receiver {
                assert_eq!(string, "Hello");
            }
        });
    });
}
