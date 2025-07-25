use std::{alloc::Layout, ptr::NonNull};

use allocator_api2::alloc::{AllocError, Allocator};

// We use duck typing instead of a trait for being generic over bump allocators
// to make it easier to work with the const generic `MIN_ALIGN`.
mod wrapper {
    pub(crate) mod bump_scope_up {
        use ::allocator_api2::alloc::Allocator;
        use ::bump_scope::{MinimumAlignment, SupportedMinimumAlignment};

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize = 1>(bump_scope::Bump<bump_scope::alloc::Global, MIN_ALIGN, true>)
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment;

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                Self(::bump_scope::Bump::new())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                Self(::bump_scope::Bump::with_size(capacity))
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &mut T {
                ::bump_scope::BumpBox::leak(self.0.alloc(value))
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&mut T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(bump_scope::BumpBox::leak(value)),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn alloc_slice_copy<T: Copy>(&self, value: &[T]) -> &mut [T] {
                self.0.alloc_slice_copy(value).into_mut()
            }

            #[inline(always)]
            pub(crate) fn try_alloc_slice_copy<T: Copy>(&self, value: &[T]) -> Option<&mut [T]> {
                match self.0.try_alloc_slice_copy(value) {
                    Ok(value) => Some(value.into_mut()),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                &self.0
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }

    pub(crate) mod bump_scope_down {
        use ::allocator_api2::alloc::Allocator;
        use ::bump_scope::{MinimumAlignment, SupportedMinimumAlignment};

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize = 1>(bump_scope::Bump<bump_scope::alloc::Global, MIN_ALIGN, false>)
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment;

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                Self(::bump_scope::Bump::new())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                Self(::bump_scope::Bump::with_size(capacity))
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &mut T {
                ::bump_scope::BumpBox::leak(self.0.alloc(value))
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&mut T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(bump_scope::BumpBox::leak(value)),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn alloc_slice_copy<T: Copy>(&self, value: &[T]) -> &mut [T] {
                self.0.alloc_slice_copy(value).into_mut()
            }

            #[inline(always)]
            pub(crate) fn try_alloc_slice_copy<T: Copy>(&self, value: &[T]) -> Option<&mut [T]> {
                match self.0.try_alloc_slice_copy(value) {
                    Ok(value) => Some(value.into_mut()),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                &self.0
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }

    pub(crate) mod bumpalo {
        use ::allocator_api2::alloc::Allocator;

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize = 1>(bumpalo::Bump<MIN_ALIGN>);

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN> {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                // NOTE: `with_min_align` is faster than `new`
                Self(::bumpalo::Bump::with_min_align())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                Self(::bumpalo::Bump::with_min_align_and_capacity(capacity))
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &mut T {
                self.0.alloc(value)
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&mut T> {
                self.0.try_alloc(value).ok()
            }

            #[inline(always)]
            pub(crate) fn alloc_slice_copy<T: Copy>(&self, value: &[T]) -> &mut [T] {
                self.0.alloc_slice_copy(value)
            }

            #[inline(always)]
            pub(crate) fn try_alloc_slice_copy<T: Copy>(&self, value: &[T]) -> Option<&mut [T]> {
                self.0.try_alloc_slice_copy(value).ok()
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                &self.0
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }

    pub(crate) mod blink_alloc {
        use core::alloc::Layout;

        use ::allocator_api2::alloc::Allocator;

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize = 1>(blink_alloc::Blink);

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN> {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                Self(::blink_alloc::Blink::new())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                let this = blink_alloc::Blink::with_chunk_size(capacity);
                // Blink does not allocate a chunk on creation.
                // We allocate here to make sure a chunk is allocated to make it fair.
                _ = this.allocator().allocate(Layout::new::<[u64; 2]>()).ok();
                Self(this)
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &mut T {
                self.0.put_no_drop(value)
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&mut T> {
                self.0.emplace_no_drop().try_value(value).ok()
            }

            #[inline(always)]
            pub(crate) fn alloc_slice_copy<T: Copy>(&self, value: &[T]) -> &mut [T] {
                self.0.copy_slice(value)
            }

            #[inline(always)]
            pub(crate) fn try_alloc_slice_copy<T: Copy>(&self, value: &[T]) -> Option<&mut [T]> {
                self.0.try_copy_slice(value)
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                self.0.allocator()
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }
}

macro_rules! benches_library {
    (
        $library:ident

        $name:ident {
            wrap($run_f:ident) {
                $($wrap:tt)*
            }
            $run_label:ident $({$($generics:tt)+})? ($($param:ident: $param_ty:ty),*) $(-> $ret:ty)? {
                $($run:tt)*
            }
        }
    ) => {
        paste::paste! {
            mod [<$name _ $library _impl>] {
                #[allow(unused_imports)]
                use crate::wrapper::$library::Bump;
                #[allow(unused_imports)]
                use crate::*;

                const _: () = {
                    // just for language server coloring
                    #[allow(dead_code)]
                    fn $run_label() {}
                };

                #[inline(never)]
                #[unsafe(no_mangle)]
                pub fn [<entry_bench_ $name _ $library>]$(<$($generics)*>)?($($param: $param_ty),*) $(-> $ret)? {
                    $($run)*
                }

                #[inline(never)]
                pub fn [<bench_ $name _ $library>](f: $(for<$($generics)*>)? fn($($param_ty),*) $(-> $ret)?) {
                    let $run_f = |$($param),*| {
                        _ = std::hint::black_box(f($(std::hint::black_box($param)),*));
                    };

                    $($wrap)*
                }
            }

            pub use [<$name _ $library _impl>]::[<entry_bench_ $name _ $library>];
            pub use [<$name _ $library _impl>]::[<bench_ $name _ $library>];
        }
    };
}

macro_rules! benches {
    ($($name:ident { $($content:tt)* })*) => {
        paste::paste! {
            $(
                const _: () = {
                    // just for language server coloring
                    mod $name {}
                };

                benches_library! {
                    bump_scope_up $name { $($content)* }
                }

                benches_library! {
                    bump_scope_down $name { $($content)* }
                }

                benches_library! {
                    bumpalo $name { $($content)* }
                }

                benches_library! {
                    blink_alloc $name { $($content)* }
                }
            )*
        }
    };
}

pub struct BigStruct(#[expect(dead_code)] [u64; 7]);

const U8_SLICE: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
const U32_SLICE: &[u32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];

impl BigStruct {
    fn new() -> Self {
        Self([0; 7])
    }
}

benches! {
    alloc_u8 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u8) -> &u8 {
            bump.alloc(value)
        }
    }

    alloc_u8_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump<8>, value: u8) -> &u8 {
            bump.alloc(value)
        }
    }

    try_alloc_u8 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u8) -> Option<&mut u8> {
            bump.try_alloc(value)
        }
    }

    try_alloc_u8_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump<8>, value: u8) -> Option<&mut u8> {
            bump.try_alloc(value)
        }
    }

    alloc_u32 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u32) -> &u32 {
            bump.alloc(value)
        }
    }

    alloc_u32_aligned {
        wrap(run) {
            let bump = Bump::<4>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump::<4>, value: u32) -> &mut u32 {
            bump.alloc(value)
        }
    }

    alloc_u32_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump::<8>, value: u32) -> &mut u32 {
            bump.alloc(value)
        }
    }

    try_alloc_u32 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u32) -> Option<&mut u32> {
            bump.try_alloc(value)
        }
    }

    try_alloc_u32_aligned {
        wrap(run) {
            let bump = Bump::<4>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump::<4>, value: u32) -> Option<&mut u32> {
            bump.try_alloc(value)
        }
    }

    try_alloc_u32_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump::<8>, value: u32) -> Option<&mut u32> {
            bump.try_alloc(value)
        }
    }

    alloc_big_struct {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, BigStruct::new());
        }
        run(bump: &Bump, value: BigStruct) -> &mut BigStruct {
            bump.alloc(value)
        }
    }

    alloc_big_struct_aligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, BigStruct::new());
        }
        run(bump: &Bump::<8>, value: BigStruct) -> &mut BigStruct {
            bump.alloc(value)
        }
    }

    alloc_big_struct_overaligned {
        wrap(run) {
            let bump = Bump::<16>::with_capacity(1024);
            run(&bump, BigStruct::new());
        }
        run(bump: &Bump::<16>, value: BigStruct) -> &mut BigStruct {
            bump.alloc(value)
        }
    }

    try_alloc_big_struct {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, BigStruct::new());
        }
        run(bump: &Bump, value: BigStruct) -> Option<&mut BigStruct> {
            bump.try_alloc(value)
        }
    }

    try_alloc_big_struct_aligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, BigStruct::new());
        }
        run(bump: &Bump::<8>, value: BigStruct) -> Option<&mut BigStruct> {
            bump.try_alloc(value)
        }
    }

    try_alloc_big_struct_overaligned {
        wrap(run) {
            let bump = Bump::<16>::with_capacity(1024);
            run(&bump, BigStruct::new());
        }
        run(bump: &Bump::<16>, value: BigStruct) -> Option<&mut BigStruct> {
            bump.try_alloc(value)
        }
    }

    alloc_u32_slice {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, U32_SLICE);
        }
        run {'a} (bump: &'a Bump, value: &[u32]) -> &'a mut [u32] {
            bump.alloc_slice_copy(value)
        }
    }

    alloc_u32_slice_aligned {
        wrap(run) {
            let bump = Bump::<4>::with_capacity(1024);
            run(&bump, U32_SLICE);
        }
        run {'a} (bump: &'a Bump::<4>, value: &[u32]) -> &'a mut [u32] {
            bump.alloc_slice_copy(value)
        }
    }

    alloc_u32_slice_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, U32_SLICE);
        }
        run {'a} (bump: &'a Bump::<8>, value: &[u32]) -> &'a mut [u32] {
            bump.alloc_slice_copy(value)
        }
    }

    try_alloc_u32_slice {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, U32_SLICE);
        }
        run {'a} (bump: &'a Bump, value: &[u32]) -> Option<&'a mut [u32]> {
            bump.try_alloc_slice_copy(value)
        }
    }

    try_alloc_u32_slice_aligned {
        wrap(run) {
            let bump = Bump::<4>::with_capacity(1024);
            run(&bump, U32_SLICE);
        }
        run {'a} (bump: &'a Bump::<4>, value: &[u32]) -> Option<&'a mut [u32]> {
            bump.try_alloc_slice_copy(value)
        }
    }

    try_alloc_u32_slice_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, U32_SLICE);
        }
        run {'a} (bump: &'a Bump::<8>, value: &[u32]) -> Option<&'a mut [u32]> {
            bump.try_alloc_slice_copy(value)
        }
    }

    alloc_u8_slice {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, U8_SLICE);
        }
        run {'a} (bump: &'a Bump, value: &[u8]) -> &'a mut [u8] {
            bump.alloc_slice_copy(value)
        }
    }

    alloc_u8_slice_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, U8_SLICE);
        }
        run {'a} (bump: &'a Bump::<8>, value: &[u8]) -> &'a mut [u8] {
            bump.alloc_slice_copy(value)
        }
    }

    try_alloc_u8_slice {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, U8_SLICE);
        }
        run {'a} (bump: &'a Bump, value: &[u8]) -> Option<&'a mut [u8]> {
            bump.try_alloc_slice_copy(value)
        }
    }

    try_alloc_u8_slice_overaligned {
        wrap(run) {
            let bump = Bump::<8>::with_capacity(1024);
            run(&bump, U8_SLICE);
        }
        run {'a} (bump: &'a Bump::<8>, value: &[u8]) -> Option<&'a mut [u8]> {
            bump.try_alloc_slice_copy(value)
        }
    }

    allocate {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump);
        }
        run(bump: &Bump) -> Result<NonNull<[u8]>, AllocError> {
            bump.as_allocator().allocate(Layout::new::<u32>())
        }
    }

    grow_same_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, Layout::new::<u32>(), Layout::new::<[u32; 2]>()) }
        }
    }

    grow_smaller_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, Layout::new::<u32>(), Layout::new::<[u16; 4]>()) }
        }
    }

    grow_larger_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, Layout::new::<u32>(), Layout::new::<u64>()) }
        }
    }

    shrink_same_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<[u32; 2]>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, Layout::new::<[u32; 2]>(), Layout::new::<u32>()) }
        }
    }

    shrink_smaller_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, Layout::new::<u32>(), Layout::new::<u16>()) }
        }
    }

    shrink_larger_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<[u16; 4]>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, Layout::new::<[u16; 4]>(), Layout::new::<u32>()) }
        }
    }

    deallocate {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) {
            unsafe { bump.as_allocator().deallocate(ptr, Layout::new::<u32>()) }
        }
    }

    deallocate_non_last {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            bump.as_allocator().allocate(Layout::new::<u32>()).unwrap();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) {
            unsafe { bump.as_allocator().deallocate(ptr, Layout::new::<u32>()) }
        }
    }

    black_box_allocate {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, Layout::new::<u32>());
        }
        run(bump: &Bump, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            bump.as_allocator().allocate(layout)
        }
    }

    black_box_grow_same_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<u32>(), Layout::new::<[u32; 2]>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, old: Layout, new: Layout) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, old, new) }
        }
    }

    black_box_grow_smaller_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<u32>(), Layout::new::<[u16; 4]>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, old: Layout, new: Layout) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, old, new) }
        }
    }

    black_box_grow_larger_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<u32>(), Layout::new::<u64>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, old: Layout, new: Layout) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, old, new) }
        }
    }

    black_box_shrink_same_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<[u32; 2]>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<[u32; 2]>(), Layout::new::<u32>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, old: Layout, new: Layout) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, old, new) }
        }
    }

    black_box_shrink_smaller_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<u32>(), Layout::new::<u16>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, old: Layout, new: Layout) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, old, new) }
        }
    }

    black_box_shrink_larger_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<[u16; 4]>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<[u16; 4]>(), Layout::new::<u32>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, old: Layout, new: Layout) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, old, new) }
        }
    }

    black_box_deallocate {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr, Layout::new::<u32>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, layout: Layout) {
            unsafe { bump.as_allocator().deallocate(ptr, layout) }
        }
    }

    black_box_deallocate_non_last {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            bump.as_allocator().allocate(Layout::new::<u32>()).unwrap();
            run(&bump, ptr, Layout::new::<u32>());
        }
        run(bump: &Bump, ptr: NonNull<u8>, layout: Layout) {
            unsafe { bump.as_allocator().deallocate(ptr, layout) }
        }
    }

    warm_up {
        wrap(run) {
            run();
        }
        run() -> Bump {
            let bump = Bump::new();
            bump.alloc(0u32);
            bump
        }
    }

    reset {
        wrap(run) {
            let mut bump = Bump::with_capacity(1024);
            bump.as_allocator().allocate(Layout::new::<u32>()).unwrap();
            run(&mut bump);
        }
        run(bump: &mut Bump) {
            bump.reset();
        }
    }
}
