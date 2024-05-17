use super::*;

either_way! {
    vec
    mut_vec
    mut_vec_rev
}

fn vec<const UP: bool>() {
    let bump = Bump::<Global, 1, UP>::new();

    bump.alloc(8u8);

    let mut vec = Vec::new_in(&bump);
    vec.push(32u32);

    let slice = vec.into_slice();
    dbg!(slice);
}

fn mut_vec<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();

    bump.alloc(8u8);

    let mut vec = MutVec::new_in(&mut bump);
    vec.push(32u32);

    let slice = vec.into_slice();
    dbg!(slice);
}

fn mut_vec_rev<const UP: bool>() {
    let mut bump = Bump::<Global, 1, UP>::new();

    bump.alloc(8u8);

    let mut vec = MutVecRev::new_in(&mut bump);
    vec.push(32u32);

    let slice = vec.into_slice();
    dbg!(slice);
}
