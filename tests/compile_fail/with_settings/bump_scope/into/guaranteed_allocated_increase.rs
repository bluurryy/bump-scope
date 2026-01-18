use bump_scope::{
    Bump, BumpScope,
    alloc::Global,
    settings::{BumpAllocatorSettings, BumpSettings},
};

type In = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<false>;
type Out = <BumpSettings as BumpAllocatorSettings>::WithGuaranteedAllocated<true>;

fn convert<'a>(bump: BumpScope<'a, Global, In>) -> BumpScope<'a, Global, Out> {
    bump.with_settings()
}

fn main() {
    let input = Bump::<Global, In>::new();

    // Can't create a non-guaranteed `BumpScope` (by value),
    // so we fake it with unsafe code.
    let input_raw = input.into_raw();

    {
        let input: BumpScope<'_, Global, In> = unsafe { BumpScope::from_raw(input_raw) };
        let output = convert(input);
        let test = output.alloc_str("test");
        println!("{test}");
    }

    unsafe { drop(Bump::<Global, In>::from_raw(input_raw)) };
}
