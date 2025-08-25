use bump_scope::{Bump, MutBumpVecRev, mut_bump_vec_rev};

pub fn triple<'a>(bump: &'a mut Bump, value: i32) -> MutBumpVecRev<i32, &'a mut Bump> {
    mut_bump_vec_rev![in bump; value; 3]
}

fn main() {}
