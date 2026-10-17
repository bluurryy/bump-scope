#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::{alloc::Global, vec::Vec};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use bump_scope::BumpPool;
use common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn rayon() {
        if cfg!(miri) {
            // rayon violates strict-provenance :(
            return;
        }

        let mut pool = BumpPool::<Global, C>::new();

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

    #[test]
    fn scope() {
        let pool = BumpPool::<Global>::new();
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
}
