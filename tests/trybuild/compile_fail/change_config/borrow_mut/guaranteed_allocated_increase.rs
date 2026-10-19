use std::alloc::Global;

use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().guaranteed_allocated(false).build();
const OUT: BumpConfig = BumpConfigBuilder::new().guaranteed_allocated(true).build();

fn convert(bump: &mut Bump<Global, IN>) -> &mut Bump<Global, OUT> {
    bump.borrow_mut_change_config()
}

fn main() {
    let mut input = Bump::<Global, IN>::new();
    let output = convert(&mut input);
    let test = output.alloc_str("test");
    println!("{test}");
}
