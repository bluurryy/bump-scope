use bump_scope::{
    Bump,
    config::{BumpConfig, BumpConfigBuilder},
};

const CONFIG: BumpConfig = BumpConfigBuilder::new().const_constructible(false).build();

fn main() {
    _ = <Bump<CONFIG>>::const_new();
}
