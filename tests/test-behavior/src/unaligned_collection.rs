use std::dbg;

use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn bump_vec() {
        let bump = Bump::<C>::new();

        bump.alloc(8u8).into_boxu();

        let mut vec = Vec::new_in(&bump);
        vec.push(32u32);

        let slice = extra::box_into_mut(vec.into_boxed_slice());
        dbg!(slice);
    }
}
