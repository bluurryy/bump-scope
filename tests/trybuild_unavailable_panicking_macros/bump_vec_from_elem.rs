use bump_scope::{Bump, BumpVec, bump_vec};

pub fn triple<'a>(bump: &'a Bump, value: i32) -> BumpVec<i32, &'a Bump> {
    bump_vec![in bump; value; 3]
}

fn main() {}
