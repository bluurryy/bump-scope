use bump_scope::{
    Bump, BumpAllocator,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().min_align(2).build();
const OUT: BumpConfig = BumpConfigBuilder::new().min_align(1).build();

fn convert(bump: &mut Bump<IN>) -> &mut Bump<OUT> {
    bump.mut_change_config()
}

fn main() {
    let mut input = Bump::<IN>::new();
    let output = convert(&mut input);
    let test = output.alloc_str("test").into_ref();
    println!("{test}");
}
