use arbitrary::Arbitrary;
use bump_scope::{BumpAllocatorExt, BumpVec, MinimumAlignment, SupportedMinimumAlignment, alloc::Global};
use zerocopy::{FromBytes, Immutable, IntoBytes};

use crate::{Bump, MinAlign};

impl Fuzz {
    pub fn run(self) {
        if self.up {
            self.run_dir::<true>();
        } else {
            self.run_dir::<false>();
        }
    }

    fn run_dir<const UP: bool>(self) {
        match self.min_align {
            MinAlign::Shl0 => self.run_dir_align::<UP, 1>(),
            MinAlign::Shl1 => self.run_dir_align::<UP, 2>(),
            MinAlign::Shl2 => self.run_dir_align::<UP, 4>(),
            MinAlign::Shl3 => self.run_dir_align::<UP, 8>(),
            MinAlign::Shl4 => self.run_dir_align::<UP, 16>(),
        }
    }

    fn run_dir_align<const UP: bool, const MIN_ALIGN: usize>(self)
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
    {
        let bump: Bump<Global, MIN_ALIGN, UP> = Bump::new();

        let mut vecs = self
            .vecs
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let pattern = (i % 255) as u8;
                match kind {
                    VecKind::T1 => VecObj::new::<T1, MIN_ALIGN, UP>(&bump, pattern),
                    VecKind::T2 => VecObj::new::<T2, MIN_ALIGN, UP>(&bump, pattern),
                    VecKind::T3 => VecObj::new::<T3, MIN_ALIGN, UP>(&bump, pattern),
                    VecKind::T4 => VecObj::new::<T4, MIN_ALIGN, UP>(&bump, pattern),
                    VecKind::T5 => VecObj::new::<T5, MIN_ALIGN, UP>(&bump, pattern),
                    VecKind::T6 => VecObj::new::<T6, MIN_ALIGN, UP>(&bump, pattern),
                }
            })
            .collect::<Vec<_>>();

        let vecs_len = vecs.len();

        if vecs_len == 0 {
            return;
        }

        for operation in self.operations {
            match operation {
                Operation::Push(i) => {
                    let vec = &mut vecs[i % vecs_len];
                    vec.push();
                    vec.assert_valid();
                }
            }

            for vec in &vecs {
                vec.assert_valid();
            }
        }
    }
}

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    up: bool,
    min_align: MinAlign,

    vecs: Vec<VecKind>,
    operations: Vec<Operation>,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum VecKind {
    T1,
    T2,
    T3,
    T4,
    T5,
    T6,
}

#[derive(Debug, Arbitrary)]

enum Operation {
    Push(usize),
}

trait VecTrait {
    fn push(&mut self, bit_pattern: u8);

    fn assert_valid(&self, bit_pattern: u8);
}

struct VecObj<'a> {
    vec: Box<dyn VecTrait + 'a>,
    bit_pattern: u8,
}

impl<'a> VecObj<'a> {
    fn new<T, const MIN_ALIGN: usize, const UP: bool>(bump: &'a Bump<Global, MIN_ALIGN, UP>, bit_pattern: u8) -> Self
    where
        MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        T: Default + FromBytes + IntoBytes + Immutable + 'static,
    {
        Self {
            vec: Box::new(BumpVec::<T, _>::new_in(bump)),
            bit_pattern,
        }
    }

    fn push(&mut self) {
        let Self { vec, bit_pattern } = self;
        vec.push(*bit_pattern);
    }

    fn assert_valid(&self) {
        let Self { vec, bit_pattern } = self;
        vec.assert_valid(*bit_pattern);
    }
}

impl<T, A> VecTrait for BumpVec<T, A>
where
    A: BumpAllocatorExt,
    T: Default + FromBytes + IntoBytes + Immutable,
{
    fn push(&mut self, bit_pattern: u8) {
        self.push(from_bit_pattern(bit_pattern));
    }

    fn assert_valid(&self, bit_pattern: u8) {
        for &byte in self.as_slice().as_bytes() {
            assert_eq!(byte, bit_pattern);
        }
    }
}

fn from_bit_pattern<T: FromBytes + IntoBytes>(byte: u8) -> T {
    let mut value = T::new_zeroed();
    value.as_mut_bytes().fill(byte);
    value
}

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
#[allow(dead_code)]
struct T1(u8);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
#[allow(dead_code)]
struct T2(u16);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
#[allow(dead_code)]
struct T3(u32);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
#[allow(dead_code)]
struct T4(u64);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
#[allow(dead_code)]
struct T5([u64; 2]);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
#[allow(dead_code)]
struct T6([u64; 3]);
