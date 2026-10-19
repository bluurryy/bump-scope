use std::alloc::Global;

use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().upwards(false).build();
const OUT: BumpConfig = BumpConfigBuilder::new().upwards(true).build();

fn convert(bump: Bump<Global, IN>) -> Bump<Global, OUT> {
    bump.change_config()
}

fn main() {
    let input = Bump::<Global, IN>::new();
    let output = convert(input);
    let test = output.alloc_str("test");
    println!("{test}");
}
