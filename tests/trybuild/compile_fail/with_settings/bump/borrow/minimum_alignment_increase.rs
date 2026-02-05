use bump_scope::{
    Bump,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<1>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<2>;

fn convert(bump: &Bump<Global, In>) -> &Bump<Global, Out> {
    bump.borrow_with_settings()
}

fn main() {
    let input = Bump::<Global, In>::new();
    let output = convert(&input);
    let test = output.alloc_str("test");
    println!("{test}");
}
