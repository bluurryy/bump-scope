use bump_scope::{Bump, BumpScope, alloc::Global, settings::BumpSettings};

fn short_type_name<T: ?Sized>() -> &'static str {
    let name = std::any::type_name::<T>();
    name.rsplit_once(':').map(|(_, rhs)| rhs).unwrap_or(name)
}

macro_rules! check_with_settings {
    ($($input:ty => $output:ty)*) => {
        const CHECK_WITH_SETTINGS: &[fn()] = &[$({
            type In = $input;
            type Out = $output;

            fn check_bump() {
                fn convert(bump: Bump<Global, In>) -> Bump<Global, Out> {
                    bump.with_settings()
                }

                let input = Bump::<Global, In>::new();
                let output = convert(input);
                let test = output.alloc_str("ok");
                print!(" {test}");
            }

            fn check_bump_scope() {
                fn convert<'a>(bump: BumpScope<'a, Global, In>) -> BumpScope<'a, Global, Out> {
                    bump.with_settings()
                }

                let mut input = Bump::<Global, In>::new();

                input.scoped(|input| {
                    let output = convert(input);
                    let test = output.alloc_str("ok");
                    print!(" {test}");
                });
            }

            fn check() {
                print!("with_settings: {} => {}\n    ",
                    short_type_name::<In>(),
                    short_type_name::<Out>(),
                );

                check_bump();
                check_bump_scope();

                println!();
            }

            check
        }),*];
    };
}

macro_rules! checks_borrow {
    ($($input:ty => $output:ty)*) => {
        const CHECKS_BORROW_WITH_SETTINGS: &[fn()] = &[$({
            type In = $input;
            type Out = $output;

            fn check_bump() {
                fn convert(bump: &Bump<Global, In>) -> &Bump<Global, Out> {
                    bump.borrow_with_settings()
                }

                let input = Bump::<Global, In>::new();
                let output = convert(&input);
                let test = output.alloc_str("ok");
                print!(" {test}");
            }

            fn check_bump_scope() {
                fn convert<'a, 'b>(bump: &'b BumpScope<'a, Global, In>) -> &'b BumpScope<'a, Global, Out> {
                    bump.borrow_with_settings()
                }

                let input = Bump::<Global, In>::new();
                let output = convert(input.as_scope());
                let test = output.alloc_str("ok");
                print!(" {test}");
            }

            fn check() {
                print!("borrow_with_settings: {} => {}\n    ",
                    short_type_name::<In>(),
                    short_type_name::<Out>(),
                );

                check_bump();
                check_bump_scope();

                println!();
            }

            check
        }),*];
    };
}

macro_rules! checks_borrow_mut {
    ($($input:ty => $output:ty)*) => {
        const CHECKS_BORROW_MUT: &[fn()] = &[$({
            type In = $input;
            type Out = $output;

            fn check_bump() {
                fn convert(bump: &mut Bump<Global, In>) -> &mut Bump<Global, Out> {
                    bump.borrow_mut_with_settings()
                }

                let mut input = Bump::<Global, In>::new();
                let output = convert(&mut input);
                let test = output.alloc_str("ok");
                print!(" {test}");
            }

            fn check_bump_scope() {
                fn convert<'a, 'b>(bump: &'b mut BumpScope<'a, Global, In>) -> &'b mut BumpScope<'a, Global, Out> {
                    bump.borrow_mut_with_settings()
                }

                let mut input = Bump::<Global, In>::new();
                let output = convert(input.as_mut_scope());
                let test = output.alloc_str("ok");
                print!(" {test}");
            }

            fn check() {
                print!("borrow_mut_with_settings: {} => {}\n    ",
                    short_type_name::<In>(),
                    short_type_name::<Out>(),
                );

                check_bump();
                check_bump_scope();

                println!();
            }

            check
        }),*];
    };
}

check_with_settings! {
    // identity
    BumpSettings => BumpSettings

    // guaranteed-allocated decrease
    BumpSettings<1, true, true> => BumpSettings<1, true, false>

    // increase and decrease minimum alignment
    BumpSettings<1> => BumpSettings<2>
    BumpSettings<2> => BumpSettings<1>
}

checks_borrow! {
    // identity
    BumpSettings => BumpSettings

    // guaranteed-allocated decrease
    BumpSettings<1, true, true> => BumpSettings<1, true, false>
}

checks_borrow_mut! {
    // identity
    BumpSettings => BumpSettings

    // increase minimum alignment
    BumpSettings<1> => BumpSettings<2>
}

fn main() {
    for checks in [CHECK_WITH_SETTINGS, CHECKS_BORROW_WITH_SETTINGS, CHECKS_BORROW_MUT] {
        for check in checks {
            check();
        }
    }
}
