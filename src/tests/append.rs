use crate::{Bump, BumpBox, BumpVec, FixedBumpVec, MutBumpVec, MutBumpVecRev};
use alloc::vec::Vec;

struct StringCounter(i32);

impl StringCounter {
    // use strings so miri catches missed or double drops
    fn inc(&mut self) -> String {
        let i = self.0;
        self.0 += 1;
        i.to_string()
    }
}

macro_rules! append {
    (
        $collector:ident;
        $(
            |$bump:ident, $items:ident| -> $ret:ty {
                $($body:tt)*
            }
        )*
    ) => {
        let mut counter = StringCounter(0);

        $({
            #[allow(unused_variables)]
            fn f($bump: &mut Bump, $items: [String; 2]) -> $ret {
                $($body)*
            }

            let mut bump: Bump = Bump::new();
            let array = [counter.inc(), counter.inc()];
            let mut by_mut = f(&mut bump, array.clone());
            assert_eq!(by_mut, array);
            $collector.append(&mut by_mut);
            assert_eq!(by_mut.len(), 0);

            let mut bump: Bump = Bump::new();
            let array = [counter.inc(), counter.inc()];
            let by_val = f(&mut bump, array.clone());
            assert_eq!(by_val, array);
            $collector.append(by_val);
        })*
    };
}

#[test]
fn append_vec() {
    let bump: Bump = Bump::new();
    let mut collector = BumpVec::new_in(&bump);

    append! {
        collector;
        |bump, items| -> Vec<String> { Vec::from_iter(items) }
        |bump, items| -> BumpBox<[String]> { bump.alloc_iter_exact(items) }
        |bump, items| -> FixedBumpVec<String> { BumpVec::from_array_in(items, bump).into_fixed_vec() }
        |bump, items| -> BumpVec<String, &Bump> { BumpVec::from_array_in(items, bump) }
        |bump, items| -> MutBumpVec<String, &mut Bump> { MutBumpVec::from_array_in(items, bump) }
        |bump, items| -> MutBumpVecRev<String, &mut Bump> { MutBumpVecRev::from_array_in(items, bump) }
    }

    assert_eq!(
        collector.as_slice(),
        (0..(4 * 6)).map(|i| i.to_string()).collect::<Vec<_>>().as_slice()
    );
}

#[test]
fn append_vec_mut() {
    let mut bump: Bump = Bump::new();
    let mut collector = MutBumpVec::new_in(&mut bump);

    append! {
        collector;
        |bump, items| -> Vec<String> { Vec::from_iter(items) }
        |bump, items| -> BumpBox<[String]> { bump.alloc_iter_exact(items) }
        |bump, items| -> FixedBumpVec<String> { BumpVec::from_array_in(items, bump).into_fixed_vec() }
        |bump, items| -> BumpVec<String, &Bump> { BumpVec::from_array_in(items, bump) }
        |bump, items| -> MutBumpVec<String, &mut Bump> { MutBumpVec::from_array_in(items, bump) }
        |bump, items| -> MutBumpVecRev<String, &mut Bump> { MutBumpVecRev::from_array_in(items, bump) }
    }

    assert_eq!(
        collector.as_slice(),
        (0..(4 * 6)).map(|i| i.to_string()).collect::<Vec<_>>().as_slice()
    );
}

#[test]
fn append_vec_mut_rev() {
    let mut bump: Bump = Bump::new();
    let mut collector = MutBumpVecRev::new_in(&mut bump);

    append! {
        collector;
        |bump, items| -> Vec<String> { Vec::from_iter(items) }
        |bump, items| -> BumpBox<[String]> { bump.alloc_iter_exact(items) }
        |bump, items| -> FixedBumpVec<String> { BumpVec::from_array_in(items, bump).into_fixed_vec() }
        |bump, items| -> BumpVec<String, &Bump> { BumpVec::from_array_in(items, bump) }
        |bump, items| -> MutBumpVec<String, &mut Bump> { MutBumpVec::from_array_in(items, bump) }
        |bump, items| -> MutBumpVecRev<String, &mut Bump> { MutBumpVecRev::from_array_in(items, bump) }
    }

    assert_eq!(
        collector.as_slice(),
        (0..(4 * 6))
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .chunks(2)
            .rev()
            .flatten()
            .cloned()
            .collect::<Vec<_>>()
            .as_slice()
    );
}
