use std::alloc::Global;

use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().claimable(true).build();
const OUT: BumpConfig = BumpConfigBuilder::new().claimable(false).build();

fn convert(bump: &mut Bump<Global, IN>) -> &mut Bump<Global, OUT> {
    bump.mut_change_config()
}

fn main() {
    let mut input = Bump::<Global, IN>::new();
    let output = convert(&mut input);
    let test = output.alloc_str("test");
    println!("{test}");
}
