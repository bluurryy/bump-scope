use std::alloc::Global;

use bump_scope::{
    Bump, BumpAllocator,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();
const OUT: BumpConfig = BumpConfigBuilder::new().const_constructible(true).build();

fn convert(bump: &mut Bump<Global, IN>) -> &mut Bump<Global, OUT> {
    bump.mut_change_config()
}

fn main() {
    let mut input = Bump::<Global, IN>::new();
    let output = convert(&mut input);
    let test = output.alloc_str("test").into_ref();
    println!("{test}");
}
