use std::alloc::Global;

use bump_scope::{
    Bump, BumpAllocator,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();

fn convert(bump: &Bump<Global, IN>) -> &Bump<Global, OUT> {
    bump.ref_change_config()
}

fn main() {
    let input = Bump::<Global, IN>::new();
    let output = convert(&input);
    let test = output.alloc_str("test").into_ref();
    println!("{test}");
}
