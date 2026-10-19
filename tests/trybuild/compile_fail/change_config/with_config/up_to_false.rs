use std::alloc::Global;

use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().upwards(true).build();
const OUT: BumpConfig = BumpConfigBuilder::new().upwards(false).build();

fn main() {
    let mut input = Bump::<Global, IN>::new();
    input.with_config::<OUT, ()>(|_| ());
}
