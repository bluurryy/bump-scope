use bump_scope::{Bump, MutBumpVec, mut_bump_vec};

pub fn triple<'a>(bump: &'a mut Bump, value: i32) -> MutBumpVec<i32, &'a mut Bump> {
    mut_bump_vec![in bump; value; 3]
}

fn main() {}
