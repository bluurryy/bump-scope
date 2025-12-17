#![no_main]

use core::{mem, ops::Range};

use fuzzing_support::{bump_scope::BumpVec, Bump};
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
        let mut vec = BumpVec::with_capacity_in(200, &bump);
        vec.extend(0..self.len);

        let expected_removed = vec[self.range.clone()].to_vec();
        let mut expected_remaining = vec[..self.range.start].to_vec();
        expected_remaining.extend(vec[self.range.end..].iter().copied());

        let original_addr = vec.as_ptr().addr();
        let removed = vec.split_off(self.range);

        assert_eq!(&*expected_removed, &*removed);
        assert_eq!(&*vec, &*expected_remaining);

        if vec.as_ptr().addr() == original_addr {
            if vec.len() < self.len {
                assert_eq!(vec.capacity(), vec.len());
            }

            assert_eq!(removed.capacity(), 200 - vec.capacity());
        } else {
            assert_eq!(removed.as_ptr().addr(), original_addr);

            if removed.len() < self.len {
                assert_eq!(removed.capacity(), removed.len());
            }

            assert_eq!(vec.capacity(), 200 - removed.capacity());
        }
    }
}
