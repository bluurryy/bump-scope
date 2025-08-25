use bump_scope::{Bump, MutBumpVec, mut_bump_vec};

pub fn one_two_three<'a>(bump: &'a mut Bump, two: i32) -> MutBumpVec<i32, &'a mut Bump> {
    mut_bump_vec![in bump; 1, two, 3]
}

fn main() {}
