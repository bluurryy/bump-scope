use std::{alloc::Layout, fmt::Debug, mem};

use arbitrary::Arbitrary;
use bump_scope::{
    alloc::Global,
    settings::{MinimumAlignment, SupportedMinimumAlignment},
    traits::BumpAllocatorTyped,
};

use crate::{Bump, MinAlign, UpTo};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    up: bool,
    min_align: MinAlign,

    offset: UpTo<17>,
    allocations: Vec<AllocationDescription>,
}

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
        let bump: Bump<Global, MIN_ALIGN, UP> = Bump::with_capacity(Layout::new::<[u8; 32]>());

        let mut prev = Range::new_slice(bump.alloc_uninit_slice::<u8>(self.offset.0).into_ref());

        for allocation in &self.allocations {
            let curr = allocation.allocate(&bump);

            let align = allocation.align() as usize;
            assert!(curr.start.is_multiple_of(align));

            prev.assert_no_overlap(curr);
            prev = curr;
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Default)]
#[expect(non_camel_case_types)]
struct t1(u8);

#[repr(transparent)]
#[derive(Clone, Default)]
#[expect(non_camel_case_types)]
struct t2(u16);

#[repr(transparent)]
#[derive(Clone, Default)]
#[expect(non_camel_case_types)]
struct t3(u32);

#[repr(transparent)]
#[derive(Clone, Default)]
#[expect(non_camel_case_types)]
struct t4(u64);

#[derive(Clone, Default)]
#[repr(align(16))]
#[expect(non_camel_case_types)]
struct t5(#[expect(dead_code)] [u8; 16]);

#[derive(Clone, Default)]
#[repr(align(32))]
#[expect(non_camel_case_types)]
struct t6(#[expect(dead_code)] [u8; 32]);

macro_rules! impl_drop {
    ($($ident:ident)*) => {
        $(
            impl Drop for $ident {
                fn drop(&mut self) {}
            }

            const _: () = assert!(core::mem::needs_drop::<$ident>());
        )*
    };
}

impl_drop! {
    t1 t2 t3 t4 t5 t6
}

#[derive(Clone, Copy)]
struct Range {
    start: usize,
    end: usize,
}

impl Range {
    fn new<T>(value: *const T) -> Self {
        let start = value.addr();
        let end = start + mem::size_of::<T>();
        Range { start, end }
    }

    fn new_slice<T>(value: *const [T]) -> Self {
        unsafe {
            let ptr = value.cast::<T>();
            let len = (&(*value)).len();

            let start = ptr.addr();
            let end = ptr.add(len).addr();
            Range { start, end }
        }
    }

    fn overlaps(self, other: Self) -> bool {
        assert!(self.start <= self.end);
        assert!(other.start <= other.end);
        self.start < other.end && other.start < self.end
    }

    fn assert_no_overlap(self, other: Self) {
        if self.overlaps(other) {
            panic!("overlap: {self:?} and {other:?}");
        }
    }
}

impl Debug for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&(self.start..self.end), f)
    }
}

trait AnyBump {
    fn alloc_dynamic(&self, layout: Layout) -> Range;
    fn alloc_static<T: Default>(&self) -> Range;
    fn alloc_static_slice<T: Default>(&self, len: usize) -> Range;
}

impl<const UP: bool, const MIN_ALIGN: usize> AnyBump for &Bump<Global, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn alloc_dynamic(&self, layout: Layout) -> Range {
        let start = self.allocate_layout(layout).addr().get();
        Range {
            start,
            end: start + layout.size(),
        }
    }

    fn alloc_static<T: Default>(&self) -> Range {
        let ptr = self.alloc_with(T::default).into_raw().as_ptr();
        Range::new(ptr)
    }

    fn alloc_static_slice<T: Default>(&self, len: usize) -> Range {
        let ptr = self.alloc_slice_fill_with(len, T::default).into_raw().as_ptr();
        Range::new_slice(ptr)
    }
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum AllocationDescription {
    Dynamic(DynamicAllocationDescription),
    Static(StaticAllocationDescription),
}

impl AllocationDescription {
    fn align(&self) -> Align {
        match self {
            AllocationDescription::Dynamic(desc) => desc.align(),
            AllocationDescription::Static(desc) => desc.align(),
        }
    }

    fn allocate(&self, bump: impl AnyBump) -> Range {
        match self {
            AllocationDescription::Dynamic(desc) => desc.allocate(bump),
            AllocationDescription::Static(desc) => desc.allocate(bump),
        }
    }
}

#[derive(Debug, Clone, Copy, Arbitrary)]
struct DynamicAllocationDescription {
    size: UpTo<17>,
    align: Align,
}

impl DynamicAllocationDescription {
    fn align(&self) -> Align {
        self.align
    }

    fn allocate(&self, bump: impl AnyBump) -> Range {
        let layout = Layout::from_size_align(self.size.0, self.align as usize).unwrap();
        bump.alloc_dynamic(layout)
    }
}

#[derive(Debug, Clone, Copy, Arbitrary)]
struct StaticAllocationDescription {
    is_slice: bool,
    stride: Align,
    count: Count,
}

impl StaticAllocationDescription {
    fn align(&self) -> Align {
        self.stride
    }

    fn allocate(&self, bump: impl AnyBump) -> Range {
        match self.is_slice {
            true => match self.stride {
                Align::T1 => bump.alloc_static_slice::<t1>(self.count as usize),
                Align::T2 => bump.alloc_static_slice::<t2>(self.count as usize),
                Align::T3 => bump.alloc_static_slice::<t3>(self.count as usize),
                Align::T4 => bump.alloc_static_slice::<t4>(self.count as usize),
                Align::T5 => bump.alloc_static_slice::<t5>(self.count as usize),
                Align::T6 => bump.alloc_static_slice::<t6>(self.count as usize),
            },
            false => match self.stride {
                Align::T1 => match self.count {
                    Count::C0 => bump.alloc_static::<[t1; 0]>(),
                    Count::C1 => bump.alloc_static::<[t1; 1]>(),
                    Count::C2 => bump.alloc_static::<[t1; 2]>(),
                    Count::C3 => bump.alloc_static::<[t1; 3]>(),
                },
                Align::T2 => match self.count {
                    Count::C0 => bump.alloc_static::<[t2; 0]>(),
                    Count::C1 => bump.alloc_static::<[t2; 1]>(),
                    Count::C2 => bump.alloc_static::<[t2; 2]>(),
                    Count::C3 => bump.alloc_static::<[t2; 3]>(),
                },
                Align::T3 => match self.count {
                    Count::C0 => bump.alloc_static::<[t3; 0]>(),
                    Count::C1 => bump.alloc_static::<[t3; 1]>(),
                    Count::C2 => bump.alloc_static::<[t3; 2]>(),
                    Count::C3 => bump.alloc_static::<[t3; 3]>(),
                },
                Align::T4 => match self.count {
                    Count::C0 => bump.alloc_static::<[t4; 0]>(),
                    Count::C1 => bump.alloc_static::<[t4; 1]>(),
                    Count::C2 => bump.alloc_static::<[t4; 2]>(),
                    Count::C3 => bump.alloc_static::<[t4; 3]>(),
                },
                Align::T5 => match self.count {
                    Count::C0 => bump.alloc_static::<[t5; 0]>(),
                    Count::C1 => bump.alloc_static::<[t5; 1]>(),
                    Count::C2 => bump.alloc_static::<[t5; 2]>(),
                    Count::C3 => bump.alloc_static::<[t5; 3]>(),
                },
                Align::T6 => match self.count {
                    Count::C0 => bump.alloc_static::<[t6; 0]>(),
                    Count::C1 => bump.alloc_static::<[t6; 1]>(),
                    Count::C2 => bump.alloc_static::<[t6; 2]>(),
                    Count::C3 => bump.alloc_static::<[t6; 3]>(),
                },
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Align {
    T1 = 1 << 0,
    T2 = 1 << 1,
    T3 = 1 << 2,
    T4 = 1 << 3,
    T5 = 1 << 4,
    T6 = 1 << 5,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Count {
    C0,
    C1,
    C2,
    C3,
}
