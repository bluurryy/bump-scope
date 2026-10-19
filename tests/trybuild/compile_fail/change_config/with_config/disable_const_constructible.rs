use std::alloc::Global;

use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();
const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();

fn main() {
    let mut input = Bump::<Global, IN>::new();
    input.with_config::<OUT, ()>(|_| ());
}
