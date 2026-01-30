use bump_scope::{
    Bump, BumpScope,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithUp<true>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithUp<false>;

fn convert(bump: BumpScope<Global, In>) -> BumpScope<Global, Out> {
    bump.with_settings()
}

fn main() {
    let mut input = Bump::<Global, In>::with_size(512);
    let mut guard = input.scope_guard();
    let output = convert(guard.scope().by_value());
    let test = output.alloc_str("test");
    println!("{test}");
}
