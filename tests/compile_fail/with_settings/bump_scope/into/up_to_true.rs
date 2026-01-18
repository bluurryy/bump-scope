use bump_scope::{
    Bump, BumpScope,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithUp<false>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithUp<true>;

fn convert<'a>(bump: BumpScope<'a, Global, In>) -> BumpScope<'a, Global, Out> {
    bump.with_settings()
}

fn main() {
    let mut input = Bump::<Global, In>::new();

    input.scoped(|input| {
        let output = convert(input);
        let test = output.alloc_str("test");
        println!("{test}");
    });
}
