use std::alloc::Global;

use bump_scope::{
    Bump, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().upwards(false).build();
const OUT: BumpConfig = BumpConfigBuilder::new().upwards(true).build();

fn convert(bump: BumpScope<Global, IN>) -> BumpScope<Global, OUT> {
    bump.change_config()
}

fn main() {
    let mut input = Bump::<Global, IN>::new();
    let mut guard = input.scope_guard();
    let output = convert(guard.scope().by_value());
    let test = output.alloc_str("test");
    println!("{test}");
}
