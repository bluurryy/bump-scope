use bump_scope::{
    Bump,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithUp<false>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithUp<true>;

fn convert(bump: &mut Bump<Global, In>) -> &mut Bump<Global, Out> {
    bump.borrow_mut_with_settings()
}

fn main() {
    let mut input = Bump::<Global, In>::new();
    let output = convert(&mut input);
    let test = output.alloc_str("test");
    println!("{test}");
}
