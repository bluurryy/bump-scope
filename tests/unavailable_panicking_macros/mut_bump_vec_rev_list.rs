use bump_scope::{Bump, MutBumpVecRev, mut_bump_vec_rev};

pub fn one_two_three<'a>(bump: &'a mut Bump, two: i32) -> MutBumpVecRev<i32, &'a mut Bump> {
    mut_bump_vec_rev![in bump; 1, two, 3]
}

fn main() {}
