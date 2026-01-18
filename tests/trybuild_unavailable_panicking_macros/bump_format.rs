use bump_scope::{Bump, BumpString, bump_format};

pub fn greet<'a>(bump: &'a Bump, greeting: &str, name: &str) -> BumpString<&'a Bump> {
    bump_format!(in bump, "{greeting}, {name}!")
}

fn main() {}
