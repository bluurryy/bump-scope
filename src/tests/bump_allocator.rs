use crate::{Bump, BumpAllocator, BumpVec, MutBumpAllocator, MutBumpVec};

fn number_strings(numbers: impl IntoIterator<Item = i32>) -> impl Iterator<Item = String> {
    numbers.into_iter().map(|i| i.to_string())
}

#[test]
fn smoke_test() {
    fn test<A: BumpAllocator>(a: A) {
        let mut vec = BumpVec::from_iter_in(number_strings(1..=5), a);
        vec.extend(number_strings(6..=9));
    }

    fn mut_test<A: MutBumpAllocator>(a: A) {
        let mut vec = MutBumpVec::from_iter_in(number_strings(1..=5), a);
        vec.extend(number_strings(6..=9));
    }

    let mut a: Bump = Bump::new();
    test(&mut a);
    test(&a);
    test(a);

    let mut a: Bump = Bump::new();
    a.scoped(|mut a| {
        test(&mut a);
        test(&a);
        test(a);
    });

    let mut a: Bump = Bump::new();
    mut_test(&mut a);
    mut_test(a);

    let mut a: Bump = Bump::new();
    a.scoped(|mut a| {
        mut_test(&mut a);
        mut_test(a);
    });
}
