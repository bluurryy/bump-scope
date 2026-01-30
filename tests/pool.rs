#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

use std::vec::Vec;

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use bump_scope::{BumpPool, alloc::Global};

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        mod up {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            )*
        }
    };
}

either_way! {
    rayon

    scope
}

fn rayon<const UP: bool>() {
    if cfg!(miri) {
        // rayon violates strict-provenance :(
        return;
    }

    let mut pool = BumpPool::<Global>::new();

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
