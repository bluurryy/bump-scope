use crate::{dbg, eprintln, MaybeFailingAllocator, RcAllocator};
use arbitrary::{Arbitrary, Unstructured};
use bump_scope::{
    allocator_api2::alloc::{Allocator, Global},
    Bump, MinimumAlignment, SupportedMinimumAlignment,
};
use core::fmt::Debug;
use rangemap::RangeSet;
use std::{alloc::Layout, ops::Range, ptr::NonNull, rc::Rc};

#[derive(Debug, Arbitrary)]
pub struct Fuzz {
    up: bool,
    min_align: MinAlign,

    operations: Vec<Operation>,
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
        dbg!(UP);
        dbg!(MIN_ALIGN);

        // We use rc to also check that allocator cloning works.
        let allocator = RcAllocator::new(Rc::new(MaybeFailingAllocator::new(Global)));
        let bump: Bump<_, MIN_ALIGN, UP> = Bump::with_capacity_in(Layout::new::<[u8; 32]>(), allocator);

        let mut allocations = vec![];
        let mut used_ranges = UsedRanges::default();

        #[allow(clippy::unused_enumerate_index)]
        for (_operation_i, &operation) in self.operations.iter().enumerate() {
            eprintln!("======================================");
            eprintln!("OPERATION {_operation_i}");
            dbg!(&allocations);
            dbg!(&used_ranges);
            dbg!(&bump);

            match operation {
                Operation::Allocate { layout, zero, fails } => unsafe {
                    let layout = layout.0;

                    bump.allocator().fails.set(fails);

                    let ptr = if zero {
                        bump.allocate_zeroed(layout)
                    } else {
                        bump.allocate(layout)
                    };

                    eprintln!("ALLOCATE");
                    dbg!(layout);
                    dbg!(&ptr);

                    if let Ok(ptr) = ptr {
                        assert_eq!(ptr.len(), layout.size());
                        assert!(ptr.is_aligned_to(layout.align()));
                        used_ranges.insert(ptr);

                        if zero {
                            assert_zeroed(ptr);
                        }

                        initialize(ptr);
                        assert_initialized(ptr);

                        allocations.push(Allocation { ptr, layout });
                    }
                },
                Operation::Deallocate { index } => unsafe {
                    if allocations.is_empty() {
                        continue;
                    }

                    let i = index % allocations.len();
                    let Allocation { ptr, layout } = allocations.swap_remove(i);

                    eprintln!("DEALLOCATE");
                    dbg!(layout);
                    dbg!(&ptr);

                    assert_eq!(ptr.len(), layout.size());
                    assert!(ptr.is_aligned_to(layout.align()));
                    used_ranges.remove(ptr);

                    assert_initialized(ptr);
                    deinitialize(ptr);

                    bump.deallocate(ptr.cast(), layout);
                },
                Operation::Reallocate {
                    index,
                    layout: new_layout,
                    zero,
                    fails,
                } => unsafe {
                    let mut new_layout = new_layout.0;

                    eprintln!("REALLOCATE");
                    dbg!(new_layout);
                    dbg!(index);

                    if allocations.is_empty() {
                        eprintln!("CANCELLED: NO ALLOCATIONS");
                        continue;
                    }

                    bump.allocator().fails.set(fails);

                    let i = index % allocations.len();
                    dbg!(i);

                    let Allocation {
                        ptr: old_ptr,
                        layout: old_layout,
                    } = allocations[i];

                    dbg!(old_layout);
                    dbg!(old_ptr);

                    assert_eq!(old_ptr.len(), old_layout.size());
                    assert!(old_ptr.is_aligned_to(old_layout.align()));
                    assert_initialized(old_ptr);

                    let new_ptr = if new_layout.size() > old_layout.size() {
                        if zero {
                            bump.grow_zeroed(old_ptr.cast(), old_layout, new_layout)
                        } else {
                            bump.grow(old_ptr.cast(), old_layout, new_layout)
                        }
                    } else {
                        bump.shrink(old_ptr.cast(), old_layout, new_layout)
                    };

                    dbg!(&new_ptr);

                    if let Ok(new_ptr) = new_ptr {
                        #[allow(ambiguous_wide_pointer_comparisons)]
                        if new_ptr == old_ptr {
                            assert_eq!(new_ptr.len(), old_layout.size());
                            new_layout = Layout::from_size_align(old_layout.size(), new_layout.align()).unwrap();
                        } else {
                            assert_eq!(new_ptr.len(), new_layout.size());
                        }

                        assert!(new_ptr.is_aligned_to(new_layout.align()));

                        if new_layout.size() > old_layout.size() {
                            let [old_part, new_part] = split_slice(new_ptr, old_layout.size());
                            assert_initialized(old_part);

                            if zero {
                                assert_zeroed(new_part);
                            }

                            initialize(new_ptr);
                        }

                        assert_initialized(new_ptr);

                        used_ranges.remove(old_ptr);
                        used_ranges.insert(new_ptr);

                        allocations[i] = Allocation {
                            ptr: new_ptr,
                            layout: new_layout,
                        }
                    }
                },
            }
        }

        eprintln!("====================================");
        eprintln!("DONE WITH ALL OPERATIONS");
        eprintln!("DROPPING REMAINING ALLOCATIONS");
        eprintln!("====================================");

        unsafe {
            for Allocation { ptr, layout } in allocations {
                dbg!(layout);
                dbg!(ptr);
                used_ranges.remove(ptr);
                assert_initialized(ptr);
                deinitialize(ptr);
                bump.deallocate(ptr.cast(), layout);
            }
        }
    }
}

#[derive(Default)]
struct UsedRanges {
    used: RangeSet<usize>,
}

impl Debug for UsedRanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut list = f.debug_list();

        for range in self.used.iter() {
            list.entry(&HexRange(range));
        }

        list.finish()
    }
}

impl UsedRanges {
    fn insert(&mut self, ptr: NonNull<[u8]>) {
        let range = addr_range(ptr);

        if !range.is_empty() {
            assert!(
                !self.used.overlaps(&range),
                "insert failed: range={:?} used={:?}",
                HexRange(&range),
                self.used
            );
            self.used.insert(range);
        }
    }

    fn remove(&mut self, ptr: NonNull<[u8]>) {
        let range = addr_range(ptr);

        if !range.is_empty() {
            assert_eq!(
                self.used.gaps(&range).map(|r| r.len()).sum::<usize>(),
                0,
                "remove failed: range={:?} used={:?}",
                HexRange(&range),
                self.used
            );
            self.used.remove(range);
        }
    }
}

struct HexRange<'a>(&'a Range<usize>);

impl Debug for HexRange<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Range { start, end } = self.0.clone();
        let len = end - start;
        write!(f, "{start:x}..{end:x} ({len})")
    }
}

fn split_slice(ptr: NonNull<[u8]>, mid: usize) -> [NonNull<[u8]>; 2] {
    assert!(mid <= ptr.len());

    let lhs_len = mid;
    let rhs_len = ptr.len() - mid;

    let lhs_ptr = ptr.cast::<u8>();
    let rhs_ptr = unsafe { ptr.cast::<u8>().add(mid) };

    [
        NonNull::slice_from_raw_parts(lhs_ptr, lhs_len),
        NonNull::slice_from_raw_parts(rhs_ptr, rhs_len),
    ]
}

fn addr_range(ptr: NonNull<[u8]>) -> Range<usize> {
    let addr = ptr.as_ptr().addr();
    addr..addr + ptr.len()
}

unsafe fn initialize(ptr: NonNull<[u8]>) {
    for i in 0..ptr.len() {
        ptr.cast::<u8>().as_ptr().add(i).write(i as u8);
    }
}

unsafe fn deinitialize(ptr: NonNull<[u8]>) {
    // write some garbage that can't be mistaken for initialized or zeroed memory
    ptr.as_ptr().cast::<u8>().write_bytes(0xFA, ptr.len())
}

unsafe fn assert_initialized(ptr: NonNull<[u8]>) {
    for i in 0..ptr.len() {
        assert_eq!(ptr.cast::<u8>().as_ptr().add(i).read(), i as u8);
    }
}

unsafe fn assert_zeroed(ptr: NonNull<[u8]>) {
    for i in 0..ptr.len() {
        assert_eq!(ptr.cast::<u8>().as_ptr().add(i).read(), 0);
    }
}

#[derive(Debug)]
struct Allocation {
    ptr: NonNull<[u8]>,
    layout: Layout,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum MinAlign {
    Shl0 = 1 << 0,
    Shl1 = 1 << 1,
    Shl2 = 1 << 2,
    Shl3 = 1 << 3,
    Shl4 = 1 << 4,
}

#[derive(Debug, Clone, Copy, Arbitrary)]
enum Operation {
    Allocate {
        layout: FuzzLayout,
        zero: bool,
        fails: bool,
    },
    Deallocate {
        index: usize,
    },
    Reallocate {
        index: usize,
        layout: FuzzLayout,
        zero: bool,
        fails: bool,
    },
}

#[derive(Debug, Clone, Copy)]
struct FuzzLayout(Layout);

impl<'a> Arbitrary<'a> for FuzzLayout {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let size = u.int_in_range(0..=512)?;
        let align = 1 << u.int_in_range(1..=8)?;
        Ok(FuzzLayout(Layout::from_size_align(size, align).unwrap()))
    }

    fn size_hint(depth: usize) -> (usize, Option<usize>) {
        <[usize; 2]>::size_hint(depth)
    }
}
