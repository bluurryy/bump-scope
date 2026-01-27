use bump_scope::{
    Bump, BumpScope,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithUp<false>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithUp<true>;

fn convert<'a, 'b>(bump: &'b mut BumpScope<'a, Global, In>) -> &'b mut BumpScope<'a, Global, Out> {
    bump.borrow_mut_with_settings()
}

fn main() {
    let mut input = Bump::<Global, In>::with_size(512);
    let output = convert(input.as_mut_scope());
    let test = output.alloc_str("test");
    println!("{test}");
}
