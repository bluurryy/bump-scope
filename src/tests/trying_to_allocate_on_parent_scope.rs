use core::{alloc::Layout, ptr::NonNull};

use crate::{
    Bump, BumpAllocator, BumpAllocatorExt, BumpBox, BumpScope, BumpVec, MinimumAlignment, SupportedMinimumAlignment,
    alloc::{AllocError, Allocator, Global},
    bump_vec,
};

// allocating on a parent scope must error, except for ZSTs
#[test]
fn allocate() {
    fn check(parent: &dyn BumpAllocator, child: &dyn BumpAllocator) {
        assert!(parent.stats().current_chunk().is_none());
        assert!(parent.allocate(Layout::new::<()>()).is_ok());
        assert!(parent.allocate(Layout::new::<u8>()).is_err());
        assert!(child.allocate(Layout::new::<()>()).is_ok());
        assert!(child.allocate(Layout::new::<u8>()).is_ok());
    }

    let parent: Bump<Global, 4> = Bump::new();
    parent.scoped(|child| check(&parent, &child));
}

// deallocating on a parent scope must do nothing
// deallocating an object from a parent scope on a child scope works
#[test]
fn deallocate() {
    const LAYOUT: Layout = Layout::new::<i32>();

    fn check(parent: &dyn BumpAllocator, child: &dyn BumpAllocator, ptr: NonNull<u8>) {
        assert!(parent.stats().current_chunk().is_none());
        assert_eq!(child.stats().allocated(), 4);
        unsafe { parent.deallocate(ptr, LAYOUT) };
        assert_eq!(child.stats().allocated(), 4);
        unsafe { child.deallocate(ptr, LAYOUT) };
        assert_eq!(child.stats().allocated(), 0);
    }

    let parent: Bump<Global, 4> = Bump::new();
    let ptr = parent.allocate(Layout::new::<i32>()).unwrap().cast();
    assert_eq!(parent.stats().allocated(), 4);
    parent.scoped(|child| check(&parent, &child, ptr));
    assert_eq!(parent.stats().allocated(), 4);
}

// growing on a parent scope must error, except for ZSTs
#[test]
fn grow() {
    struct Input {
        old_layout: Layout,
        new_layout: Layout,
        allocates: usize,
        errors: bool,
    }

    fn check(
        Input {
            old_layout,
            new_layout,
            allocates,
            errors,
        }: Input,
    ) {
        let parent: Bump = Bump::new();
        parent.allocate(Layout::new::<u8>()).unwrap(); // mess up alignment
        let ptr = parent.allocate(old_layout).unwrap().cast();
        parent.scoped(|child| {
            assert!(parent.stats().current_chunk().is_none());
            assert_eq!(child.stats().allocated(), allocates);
            let result = unsafe { parent.grow(ptr, old_layout, new_layout) };
            assert_eq!(result.is_err(), errors);
            assert_eq!(child.stats().allocated(), allocates);
        });
    }

    // in place
    check(Input {
        old_layout: Layout::new::<[u32; 1]>(),
        new_layout: Layout::new::<[u32; 2]>(),
        allocates: 8,
        errors: true,
    });

    // not in place
    check(Input {
        old_layout: Layout::new::<[u8; 1]>(),
        new_layout: Layout::new::<[u64; 1]>(),
        allocates: 2,
        errors: true,
    });

    // zst
    check(Input {
        old_layout: Layout::new::<[u8; 0]>(),
        new_layout: Layout::new::<[u8; 0]>(),
        allocates: 1,
        errors: false,
    });

    // in another chunk
    check(Input {
        old_layout: Layout::new::<[u8; 1]>(),
        new_layout: Layout::new::<[u8; 512]>(),
        allocates: 2,
        errors: true,
    });
}

// shrinking on a parent scope must do nothing
#[test]
fn shrink() {
    struct Input {
        old_layout: Layout,
        new_layout: Layout,
        allocates: usize,
        errors: bool,
    }

    fn check(
        Input {
            old_layout,
            new_layout,
            allocates,
            errors,
        }: Input,
    ) {
        let parent: Bump = Bump::new();
        parent.allocate(Layout::new::<u8>()).unwrap(); // mess up alignment
        let ptr = parent.allocate(old_layout).unwrap().cast();
        parent.scoped(|child| {
            assert!(parent.stats().current_chunk().is_none());
            assert_eq!(child.stats().allocated(), allocates);
            let result = unsafe { parent.shrink(ptr, old_layout, new_layout) };
            assert_eq!(result.is_err(), errors);
            assert_eq!(child.stats().allocated(), allocates);
        });
    }

    // in place, does nothing
    check(Input {
        old_layout: Layout::new::<[u32; 3]>(),
        new_layout: Layout::new::<[u32; 2]>(),
        allocates: 16,
        errors: false,
    });

    // would require allocation, errors
    check(Input {
        old_layout: Layout::new::<[u8; 5]>(),
        new_layout: Layout::new::<u32>(),
        allocates: 6,
        errors: true,
    });

    // zst, does nothing
    check(Input {
        old_layout: Layout::new::<[u8; 5]>(),
        new_layout: Layout::new::<[u32; 0]>(),
        allocates: 6,
        errors: false,
    });
}
