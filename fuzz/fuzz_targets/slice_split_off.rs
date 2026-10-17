#![no_main]

use core::{mem, ops::Range};

use fuzzing_support::bump_scope::Bump;
use libfuzzer_sys::{
    arbitrary::{Arbitrary, Unstructured},
    fuzz_target,
};

fuzz_target!(|fuzz: Fuzz| fuzz.run());

#[derive(Debug)]
struct Fuzz {
    len: usize,
    range: Range<usize>,
}

impl<'a> Arbitrary<'a> for Fuzz {
    fn arbitrary(u: &mut Unstructured<'a>) -> libfuzzer_sys::arbitrary::Result<Self> {
        let len = u.int_in_range(0..=10)?;
        let mut start = u.int_in_range(0..=len)?;
        let mut end = u.int_in_range(0..=len)?;

        if start > end {
            mem::swap(&mut start, &mut end);
        }

        Ok(Fuzz { len, range: start..end })
    }
}

impl Fuzz {
    fn run(self) {
        let bump: Bump = Bump::new();
        let mut slice = bump.alloc_iter(0..self.len);

        let expected_removed = (&*slice)[self.range.clone()].to_vec();
        let mut expected_remaining = (&*slice)[..self.range.start].to_vec();
        expected_remaining.extend((&*slice)[self.range.end..].iter().copied());

        let removed = slice.split_off(self.range);

        assert_eq!(&*expected_removed, &*removed);
        assert_eq!(&*slice, &*expected_remaining);
    }
}
