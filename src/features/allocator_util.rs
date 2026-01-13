// spellchecker:off because of "othr"
macro_rules! allocator_compat_wrapper {
    (
        $(#[$attr:meta])*
        struct $struct:ident for $othr:ident
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(Debug, Default, Clone)]
        pub struct $struct<A: ?Sized>(pub A);

        impl<A: ?Sized> $struct<A> {
            #[inline(always)]
            #[expect(missing_docs)]
            pub fn from_ref(allocator: &A) -> &Self {
                unsafe { &*(core::ptr::from_ref(allocator) as *const Self) }
            }

            #[inline(always)]
            #[expect(missing_docs)]
            pub fn from_mut(allocator: &mut A) -> &mut Self {
                unsafe { &mut *(core::ptr::from_mut(allocator) as *mut Self) }
            }
        }

        impl_allocator_via_allocator! {
            self;

            use {&self.0} for crate as $othr impl[A: ?Sized + $othr::alloc::Allocator] $struct<A>
            use {&self.0} for $othr as crate impl[A: ?Sized + crate::alloc::Allocator] $struct<A>
        }

        #[test]
        fn test_compat() {
            use core::{alloc::Layout, ptr::NonNull};

            use crate::{BaseAllocator, settings::True};

            #[derive(Clone)]
            struct OthrAllocator;

            unsafe impl $othr::alloc::Allocator for OthrAllocator {
                fn allocate(&self, _: Layout) -> Result<NonNull<[u8]>, $othr::alloc::AllocError> {
                    unimplemented!()
                }

                unsafe fn deallocate(&self, _: NonNull<u8>, _: Layout) {
                    unimplemented!()
                }
            }

            fn is_base_allocator<T: BaseAllocator<True>>(_: T) {}

            #[cfg(feature = "alloc")]
            is_base_allocator(Global);
            is_base_allocator($struct(OthrAllocator));
            is_base_allocator($struct::from_ref(&OthrAllocator));
        }
    };
}
// spellchecker:on

macro_rules! impl_allocator_via_allocator {
    (
        $self:ident;
        $(
            $(#[$attr:meta])*
            use {$accessor:expr} for $target_crate:ident as $source_crate:ident
            impl [$($($args:tt)+)?]
            $ty:ty
            $(where [$($bounds:tt)*])?
        )*
    ) => {
        $(
            const _: () = {
                use core::{alloc::Layout, ptr::NonNull};

                use $target_crate::alloc::{
                    Allocator as TargetAllocator,
                    AllocError as TargetAllocError,
                };

                use $source_crate::alloc::{
                    Allocator as SourceAllocator,
                };

                $(#[$attr])*
                unsafe impl $(<$($args)*>)? TargetAllocator for $ty
                $(where $($bounds)*)?
                {
                    #[inline(always)]
                    fn allocate(&$self, layout: Layout) -> Result<NonNull<[u8]>, TargetAllocError> {
                        SourceAllocator::allocate($accessor, layout).map_err(Into::into)
                    }

                    #[inline(always)]
                    unsafe fn deallocate(&$self, ptr: NonNull<u8>, layout: Layout) {
                        unsafe { SourceAllocator::deallocate($accessor, ptr, layout) };
                    }

                    #[inline(always)]
                    unsafe fn grow(
                        &$self,
                        ptr: NonNull<u8>,
                        old_layout: Layout,
                        new_layout: Layout,
                    ) -> Result<NonNull<[u8]>, TargetAllocError> {
                        unsafe { SourceAllocator::grow($accessor, ptr, old_layout, new_layout).map_err(Into::into) }
                    }

                    #[inline(always)]
                    unsafe fn grow_zeroed(
                        &$self,
                        ptr: NonNull<u8>,
                        old_layout: Layout,
                        new_layout: Layout,
                    ) -> Result<NonNull<[u8]>, TargetAllocError> {
                        unsafe {
                            SourceAllocator::grow_zeroed($accessor, ptr, old_layout, new_layout).map_err(Into::into)
                        }
                    }

                    #[inline(always)]
                    unsafe fn shrink(
                        &$self,
                        ptr: NonNull<u8>,
                        old_layout: Layout,
                        new_layout: Layout,
                    ) -> Result<NonNull<[u8]>, TargetAllocError> {
                        unsafe { SourceAllocator::shrink($accessor, ptr, old_layout, new_layout).map_err(Into::into) }
                    }
                }
            };
        )*
    };
}

pub(crate) use allocator_compat_wrapper;
pub(crate) use impl_allocator_via_allocator;
