use bump_scope::{
    Bump, BumpScope,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<2>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithMinimumAlignment<1>;

fn convert<'a, 'b>(bump: &'b BumpScope<'a, Global, In>) -> &'b BumpScope<'a, Global, Out> {
    bump.borrow_with_settings()
}

fn main() {
    let input = Bump::<Global, In>::with_size(512);
    let output = convert(input.as_scope());
    let test = output.alloc_str("test");
    println!("{test}");
}
