use bump_scope::{
    Bump,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<2>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<1>;

fn convert(bump: &mut Bump<Global, In>) -> &mut Bump<Global, Out> {
    bump.borrow_mut_with_settings()
}

fn main() {
    let mut input = Bump::<Global, In>::new();
    let output = convert(&mut input);
    let test = output.alloc_str("test");
    println!("{test}");
}
