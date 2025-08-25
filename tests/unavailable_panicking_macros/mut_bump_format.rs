use bump_scope::{Bump, MutBumpString, mut_bump_format};

pub fn greet<'a>(bump: &'a mut Bump, greeting: &str, name: &str) -> MutBumpString<&'a mut Bump> {
    mut_bump_format!(in bump, "{greeting}, {name}!")
}

fn main() {}
