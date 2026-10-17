use std::alloc::Global;

use arbitrary::Arbitrary;
use bump_scope::{
    Bump, BumpVec,
    config::{BumpConfig, BumpConfigBuilder},
    traits::BumpAllocatorTyped,
};
use zerocopy::{FromBytes, Immutable, IntoBytes};

use crate::MinAlign;

macro_rules! run_dir {
    ($self:ident, $upwards:literal) => {
        match $self.min_align {
            MinAlign::Shl0 => run_dir_align!($self, $upwards, 1),
            MinAlign::Shl1 => run_dir_align!($self, $upwards, 2),
            MinAlign::Shl2 => run_dir_align!($self, $upwards, 4),
            MinAlign::Shl3 => run_dir_align!($self, $upwards, 8),
            MinAlign::Shl4 => run_dir_align!($self, $upwards, 16),
        }
    };
}

macro_rules! run_dir_align {
    ($self:ident, $upwards:literal, $min_align:literal) => {{
        const C: BumpConfig = BumpConfigBuilder::new().upwards($upwards).min_align($min_align).build();
        let bump = Bump::<Global, C>::new();

        let mut vecs = $self
            .vecs
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let pattern = (i % 255) as u8;
                match kind {
                    VecKind::T1 => VecObj::new::<T1, C>(&bump, pattern),
                    VecKind::T2 => VecObj::new::<T2, C>(&bump, pattern),
                    VecKind::T3 => VecObj::new::<T3, C>(&bump, pattern),
                    VecKind::T4 => VecObj::new::<T4, C>(&bump, pattern),
                    VecKind::T5 => VecObj::new::<T5, C>(&bump, pattern),
                    VecKind::T6 => VecObj::new::<T6, C>(&bump, pattern),
                }
            })
            .collect::<Vec<_>>();

        let vecs_len = vecs.len();

        if vecs_len == 0 {
            return;
        }

        for operation in $self.operations {
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
    }};
}

impl Fuzz {
    pub fn run(self) {
        if self.up {
            run_dir!(self, true);
        } else {
            run_dir!(self, false);
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
    fn new<T, const C: BumpConfig>(bump: &'a Bump<Global, C>, bit_pattern: u8) -> Self
    where
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
    A: BumpAllocatorTyped,
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
struct T1(u8);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
struct T2(u16);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
struct T3(u32);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
struct T4(u64);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
struct T5([u64; 2]);

#[repr(transparent)]
#[derive(Clone, Default, IntoBytes, FromBytes, Immutable)]
struct T6([u64; 3]);
