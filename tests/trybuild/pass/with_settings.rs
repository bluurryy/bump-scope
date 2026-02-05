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

            fn check() {
                print!("with_settings: {} => {} ",
                    short_type_name::<In>(),
                    short_type_name::<Out>(),
                );

                check_bump();

                println!();
            }

            check
        }),*];
    };
}

macro_rules! check_scope_with_settings {
    ($($input:ty => $output:ty)*) => {
        const CHECK_SCOPE_WITH_SETTINGS: &[fn()] = &[$({
            type In = $input;
            type Out = $output;

            fn check_bump_scope() {
                fn convert<'a>(bump: BumpScope<'a, Global, In>) -> BumpScope<'a, Global, Out> {
                    bump.with_settings()
                }

                let mut input = Bump::<Global, In>::new();

                input.scoped(|input| {
                    let output = convert(input.by_value());
                    let test = output.alloc_str("ok");
                    print!(" {test}");
                });
            }

            fn check() {
                print!("scope_with_settings: {} => {} ",
                    short_type_name::<In>(),
                    short_type_name::<Out>(),
                );

                check_bump_scope();

                println!();
            }

            check
        }),*];
    };
}

macro_rules! check_borrow {
    ($($input:ty => $output:ty)*) => {
        const CHECK_BORROW_WITH_SETTINGS: &[fn()] = &[$({
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
                print!("borrow_with_settings: {} => {} ",
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

macro_rules! check_borrow_mut {
    ($($input:ty => $output:ty)*) => {
        const CHECK_BORROW_MUT: &[fn()] = &[$({
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
                print!("borrow_mut_with_settings: {} => {} ",
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

    // guaranteed-allocated increase and decrease
    BumpSettings<1, true, false> => BumpSettings<1, true, true>
    BumpSettings<1, true, true> => BumpSettings<1, true, false>

    // claimable increase and decrease
    BumpSettings<1, true, true, false> => BumpSettings<1, true, true, true>
    BumpSettings<1, true, true, true> => BumpSettings<1, true, true, false>

    // increase and decrease minimum alignment
    BumpSettings<1> => BumpSettings<2>
    BumpSettings<2> => BumpSettings<1>
}

check_scope_with_settings! {
    // identity
    BumpSettings => BumpSettings

    // guaranteed-allocated increase and decrease
    BumpSettings<1, true, false> => BumpSettings<1, true, true>
    BumpSettings<1, true, true> => BumpSettings<1, true, false>

    // increase minimum alignment
    BumpSettings<1> => BumpSettings<2>
}

check_borrow! {
    // identity
    BumpSettings => BumpSettings

    // guaranteed-allocated decrease
    BumpSettings<1, true, true> => BumpSettings<1, true, false>
}

check_borrow_mut! {
    // identity
    BumpSettings => BumpSettings

    // increase minimum alignment
    BumpSettings<1> => BumpSettings<2>
}

fn main() {
    for checks in [
        CHECK_WITH_SETTINGS,
        CHECK_SCOPE_WITH_SETTINGS,
        CHECK_BORROW_WITH_SETTINGS,
        CHECK_BORROW_MUT,
    ] {
        for check in checks {
            check();
        }
    }
}
