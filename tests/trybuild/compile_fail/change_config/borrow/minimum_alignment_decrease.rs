use std::alloc::Global;

use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().min_align(2).build();
const OUT: BumpConfig = BumpConfigBuilder::new().min_align(1).build();

fn convert(bump: &Bump<Global, IN>) -> &Bump<Global, OUT> {
    bump.borrow_change_config()
}

fn main() {
    let input = Bump::<Global, IN>::new();
    let output = convert(&input);
    let test = output.alloc_str("test");
    println!("{test}");
}
