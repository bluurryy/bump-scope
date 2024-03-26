use allocator_api2::vec::Vec;

use crate::Bump;

use super::either_way;

either_way! {
    grow

    shrink

    shrinknt
}

fn grow<const UP: bool>() {
    let mut bump: Bump<1, UP> = Bump::new();

    let mut vec = Vec::<i32, _>::new_in(&bump);
    assert_eq!(bump.stats().allocated(), 0);

    vec.reserve_exact(1);
    assert_eq!(bump.stats().allocated(), 4);

    vec.reserve_exact(2);
    assert_eq!(bump.stats().allocated(), 8);

    bump.alloc_uninit::<u8>();
    assert_eq!(bump.stats().allocated(), 9);

    vec.reserve_exact(3);
    assert_eq!(bump.stats().allocated(), 12 + 3 * 4);

    drop(vec);
    assert_eq!(bump.stats().allocated(), 12);

    bump.reset();
    assert_eq!(bump.stats().allocated(), 0);
}

fn shrink<const UP: bool>() {
    let bump: Bump<1, UP> = Bump::new();

    let mut vec = Vec::<i32, _>::new_in(&bump);
    assert_eq!(bump.stats().allocated(), 0);

    vec.reserve_exact(2);
    assert_eq!(bump.stats().allocated(), 8);

    vec.shrink_to(1);
    assert_eq!(bump.stats().allocated(), 4);

    let boxed = bump.alloc_uninit::<u8>().into_box(&bump);
    assert_eq!(bump.stats().allocated(), 5);

    drop(boxed);
    assert_eq!(bump.stats().allocated(), 4);

    vec.shrink_to_fit();
    assert_eq!(bump.stats().allocated(), 0);
}

fn shrinknt<const UP: bool>() {
    let bump: Bump<1, UP> = Bump::new();

    let mut vec = Vec::<i32, _>::new_in(&bump);
    assert_eq!(bump.stats().allocated(), 0);

    vec.reserve_exact(2);
    assert_eq!(bump.stats().allocated(), 8);

    bump.alloc_uninit::<u8>();
    assert_eq!(bump.stats().allocated(), 9);

    vec.shrink_to(1);
    assert_eq!(bump.stats().allocated(), 9);

    drop(vec);
    assert_eq!(bump.stats().allocated(), 9);
}
