use bump_scope::{Bump, BumpVec, bump_vec};

pub fn one_two_three<'a>(bump: &'a Bump, two: i32) -> BumpVec<i32, &'a Bump> {
    bump_vec![in bump; 1, two, 3]
}

fn main() {}
