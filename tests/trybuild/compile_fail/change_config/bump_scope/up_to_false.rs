use bump_scope::{
    Bump, BumpAllocator, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
};

const IN: BumpConfig = BumpConfigBuilder::new().upwards(true).build();
const OUT: BumpConfig = BumpConfigBuilder::new().upwards(false).build();

fn convert(bump: BumpScope<IN>) -> BumpScope<OUT> {
    bump.change_config()
}

fn main() {
    let mut input = Bump::<IN>::new();
    let mut guard = input.scope_guard();
    let output = convert(guard.scope().by_value());
    let test = output.alloc_str("test").into_ref();
    println!("{test}");
}
