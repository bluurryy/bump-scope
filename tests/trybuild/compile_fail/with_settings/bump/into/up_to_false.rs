use bump_scope::{
    Bump,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithUp<true>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithUp<false>;

fn convert(bump: Bump<Global, In>) -> Bump<Global, Out> {
    bump.with_settings()
}

fn main() {
    let input = Bump::<Global, In>::with_size(512);
    let output = convert(input);
    let test = output.alloc_str("test");
    println!("{test}");
}
