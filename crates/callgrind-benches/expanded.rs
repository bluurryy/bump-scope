#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2024::*;
#[macro_use]
extern crate std;
use std::{alloc::Layout, ptr::NonNull};
use allocator_api2::alloc::{AllocError, Allocator};
mod wrapper {
    pub(crate) mod bump_scope_up {
        use ::allocator_api2::alloc::Allocator;
        use ::bump_scope::{MinimumAlignment, SupportedMinimumAlignment};
        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize = 1>(
            bump_scope::Bump<bump_scope::alloc::Global, MIN_ALIGN, true>,
        )
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
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                ::bump_scope::BumpBox::leak(self.0.alloc(value))
            }
            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(bump_scope::BumpBox::leak(value)),
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
        pub struct Bump<const MIN_ALIGN: usize = 1>(
            bump_scope::Bump<bump_scope::alloc::Global, MIN_ALIGN, false>,
        )
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
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                ::bump_scope::BumpBox::leak(self.0.alloc(value))
            }
            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(bump_scope::BumpBox::leak(value)),
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
                Self(::bumpalo::Bump::with_min_align())
            }
            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                Self(::bumpalo::Bump::with_min_align_and_capacity(capacity))
            }
            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                self.0.alloc(value)
            }
            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(value),
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
                _ = this.allocator().allocate(Layout::new::<[u64; 2]>()).ok();
                Self(this)
            }
            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                self.0.put_no_drop(value)
            }
            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.emplace_no_drop().try_value(value) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
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
pub mod bench_alloc_u8 {
    pub mod alloc_u8 {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_alloc_u32 {
    pub mod alloc_u32 {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_alloc_u32_aligned {
    pub mod alloc_u32_aligned {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_try_alloc_u32 {
    pub mod try_alloc_u32 {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_try_alloc_u32_aligned {
    pub mod try_alloc_u32_aligned {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_allocate {
    pub mod allocate {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_grow_same_align {
    pub mod grow_same_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_grow_smaller_align {
    pub mod grow_smaller_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_grow_larger_align {
    pub mod grow_larger_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_shrink_same_align {
    pub mod shrink_same_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_shrink_smaller_align {
    pub mod shrink_smaller_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_shrink_larger_align {
    pub mod shrink_larger_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_deallocate {
    pub mod deallocate {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_deallocate_non_last {
    pub mod deallocate_non_last {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_allocate {
    pub mod black_box_allocate {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_grow_same_align {
    pub mod black_box_grow_same_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_grow_smaller_align {
    pub mod black_box_grow_smaller_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_grow_larger_align {
    pub mod black_box_grow_larger_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_shrink_same_align {
    pub mod black_box_shrink_same_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_shrink_smaller_align {
    pub mod black_box_shrink_smaller_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_shrink_larger_align {
    pub mod black_box_shrink_larger_align {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_deallocate {
    pub mod black_box_deallocate {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_black_box_deallocate_non_last {
    pub mod black_box_deallocate_non_last {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_warm_up {
    pub mod warm_up {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
pub mod bench_reset {
    pub mod reset {
        use super::*;
        pub const __BENCHES: &[&(
            &'static str,
            fn() -> Option<::iai_callgrind::__internal::InternalLibraryBenchmarkConfig>,
            &[::iai_callgrind::__internal::InternalMacroLibBench],
        )] = &[
            &(
                "bump_scope_up",
                super::bump_scope_up::__get_config,
                super::bump_scope_up::__BENCHES,
            ),
            &(
                "bump_scope_down",
                super::bump_scope_down::__get_config,
                super::bump_scope_down::__BENCHES,
            ),
            &("bumpalo", super::bumpalo::__get_config, super::bumpalo::__BENCHES),
            &(
                "blink_alloc",
                super::blink_alloc::__get_config,
                super::blink_alloc::__BENCHES,
            ),
        ];
        #[inline(never)]
        pub fn __get_config() -> Option<
            ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
        > {
            let mut config: Option<
                ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
            > = None;
            config
        }
        #[inline(never)]
        pub fn __compare_by_id() -> Option<bool> {
            let mut comp = None;
            comp
        }
        #[inline(never)]
        pub fn __run_setup(__run: bool) -> bool {
            let mut __has_setup = false;
            __has_setup
        }
        #[inline(never)]
        pub fn __run_teardown(__run: bool) -> bool {
            let mut __has_teardown = false;
            __has_teardown
        }
        #[inline(never)]
        pub fn __run(group_index: usize, bench_index: usize) {
            (__BENCHES[group_index].2[bench_index].func)();
        }
    }
}
use bench_alloc_u8::alloc_u8;
use bench_alloc_u32::alloc_u32;
use bench_alloc_u32_aligned::alloc_u32_aligned;
use bench_try_alloc_u32::try_alloc_u32;
use bench_try_alloc_u32_aligned::try_alloc_u32_aligned;
use bench_allocate::allocate;
use bench_grow_same_align::grow_same_align;
use bench_grow_smaller_align::grow_smaller_align;
use bench_grow_larger_align::grow_larger_align;
use bench_shrink_same_align::shrink_same_align;
use bench_shrink_smaller_align::shrink_smaller_align;
use bench_shrink_larger_align::shrink_larger_align;
use bench_deallocate::deallocate;
use bench_deallocate_non_last::deallocate_non_last;
use bench_black_box_allocate::black_box_allocate;
use bench_black_box_grow_same_align::black_box_grow_same_align;
use bench_black_box_grow_smaller_align::black_box_grow_smaller_align;
use bench_black_box_grow_larger_align::black_box_grow_larger_align;
use bench_black_box_shrink_same_align::black_box_shrink_same_align;
use bench_black_box_shrink_smaller_align::black_box_shrink_smaller_align;
use bench_black_box_shrink_larger_align::black_box_shrink_larger_align;
use bench_black_box_deallocate::black_box_deallocate;
use bench_black_box_deallocate_non_last::black_box_deallocate_non_last;
use bench_warm_up::warm_up;
use bench_reset::reset;
#[inline(never)]
fn __run() {
    let mut this_args = std::env::args();
    let mut runner = ::iai_callgrind::__internal::Runner::new(
        ::core::option::Option::None::<&'static str>
            .or_else(|| ::core::option::Option::None::<&'static str>),
        &::iai_callgrind::__internal::BenchmarkKind::LibraryBenchmark,
        "/home/z/dev/bump-scope/crates/callgrind-benches",
        "callgrind-benches",
        "benches/bench.rs",
        "bench",
        this_args.next().unwrap(),
    );
    let mut config: Option<
        ::iai_callgrind::__internal::InternalLibraryBenchmarkConfig,
    > = None;
    let mut internal_benchmark_groups = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroups {
        config: config.unwrap_or_default(),
        command_line_args: this_args.collect(),
        has_setup: __run_setup(false),
        has_teardown: __run_teardown(false),
        ..Default::default()
    };
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "alloc_u8".to_owned(),
        config: alloc_u8::__get_config(),
        compare_by_id: alloc_u8::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: alloc_u8::__run_setup(false),
        has_teardown: alloc_u8::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in alloc_u8::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "alloc_u32".to_owned(),
        config: alloc_u32::__get_config(),
        compare_by_id: alloc_u32::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: alloc_u32::__run_setup(false),
        has_teardown: alloc_u32::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in alloc_u32::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "alloc_u32_aligned".to_owned(),
        config: alloc_u32_aligned::__get_config(),
        compare_by_id: alloc_u32_aligned::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: alloc_u32_aligned::__run_setup(false),
        has_teardown: alloc_u32_aligned::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in alloc_u32_aligned::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "try_alloc_u32".to_owned(),
        config: try_alloc_u32::__get_config(),
        compare_by_id: try_alloc_u32::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: try_alloc_u32::__run_setup(false),
        has_teardown: try_alloc_u32::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in try_alloc_u32::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "try_alloc_u32_aligned".to_owned(),
        config: try_alloc_u32_aligned::__get_config(),
        compare_by_id: try_alloc_u32_aligned::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: try_alloc_u32_aligned::__run_setup(false),
        has_teardown: try_alloc_u32_aligned::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in try_alloc_u32_aligned::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "allocate".to_owned(),
        config: allocate::__get_config(),
        compare_by_id: allocate::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: allocate::__run_setup(false),
        has_teardown: allocate::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in allocate::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "grow_same_align".to_owned(),
        config: grow_same_align::__get_config(),
        compare_by_id: grow_same_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: grow_same_align::__run_setup(false),
        has_teardown: grow_same_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in grow_same_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "grow_smaller_align".to_owned(),
        config: grow_smaller_align::__get_config(),
        compare_by_id: grow_smaller_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: grow_smaller_align::__run_setup(false),
        has_teardown: grow_smaller_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in grow_smaller_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "grow_larger_align".to_owned(),
        config: grow_larger_align::__get_config(),
        compare_by_id: grow_larger_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: grow_larger_align::__run_setup(false),
        has_teardown: grow_larger_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in grow_larger_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "shrink_same_align".to_owned(),
        config: shrink_same_align::__get_config(),
        compare_by_id: shrink_same_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: shrink_same_align::__run_setup(false),
        has_teardown: shrink_same_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in shrink_same_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "shrink_smaller_align".to_owned(),
        config: shrink_smaller_align::__get_config(),
        compare_by_id: shrink_smaller_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: shrink_smaller_align::__run_setup(false),
        has_teardown: shrink_smaller_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in shrink_smaller_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "shrink_larger_align".to_owned(),
        config: shrink_larger_align::__get_config(),
        compare_by_id: shrink_larger_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: shrink_larger_align::__run_setup(false),
        has_teardown: shrink_larger_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in shrink_larger_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "deallocate".to_owned(),
        config: deallocate::__get_config(),
        compare_by_id: deallocate::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: deallocate::__run_setup(false),
        has_teardown: deallocate::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in deallocate::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "deallocate_non_last".to_owned(),
        config: deallocate_non_last::__get_config(),
        compare_by_id: deallocate_non_last::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: deallocate_non_last::__run_setup(false),
        has_teardown: deallocate_non_last::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in deallocate_non_last::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_allocate".to_owned(),
        config: black_box_allocate::__get_config(),
        compare_by_id: black_box_allocate::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_allocate::__run_setup(false),
        has_teardown: black_box_allocate::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_allocate::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_grow_same_align".to_owned(),
        config: black_box_grow_same_align::__get_config(),
        compare_by_id: black_box_grow_same_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_grow_same_align::__run_setup(false),
        has_teardown: black_box_grow_same_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_grow_same_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_grow_smaller_align".to_owned(),
        config: black_box_grow_smaller_align::__get_config(),
        compare_by_id: black_box_grow_smaller_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_grow_smaller_align::__run_setup(false),
        has_teardown: black_box_grow_smaller_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_grow_smaller_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_grow_larger_align".to_owned(),
        config: black_box_grow_larger_align::__get_config(),
        compare_by_id: black_box_grow_larger_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_grow_larger_align::__run_setup(false),
        has_teardown: black_box_grow_larger_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_grow_larger_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_shrink_same_align".to_owned(),
        config: black_box_shrink_same_align::__get_config(),
        compare_by_id: black_box_shrink_same_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_shrink_same_align::__run_setup(false),
        has_teardown: black_box_shrink_same_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_shrink_same_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_shrink_smaller_align".to_owned(),
        config: black_box_shrink_smaller_align::__get_config(),
        compare_by_id: black_box_shrink_smaller_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_shrink_smaller_align::__run_setup(false),
        has_teardown: black_box_shrink_smaller_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_shrink_smaller_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_shrink_larger_align".to_owned(),
        config: black_box_shrink_larger_align::__get_config(),
        compare_by_id: black_box_shrink_larger_align::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_shrink_larger_align::__run_setup(false),
        has_teardown: black_box_shrink_larger_align::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_shrink_larger_align::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_deallocate".to_owned(),
        config: black_box_deallocate::__get_config(),
        compare_by_id: black_box_deallocate::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_deallocate::__run_setup(false),
        has_teardown: black_box_deallocate::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_deallocate::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "black_box_deallocate_non_last".to_owned(),
        config: black_box_deallocate_non_last::__get_config(),
        compare_by_id: black_box_deallocate_non_last::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: black_box_deallocate_non_last::__run_setup(false),
        has_teardown: black_box_deallocate_non_last::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in black_box_deallocate_non_last::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "warm_up".to_owned(),
        config: warm_up::__get_config(),
        compare_by_id: warm_up::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: warm_up::__run_setup(false),
        has_teardown: warm_up::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in warm_up::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let mut internal_group = ::iai_callgrind::__internal::InternalLibraryBenchmarkGroup {
        id: "reset".to_owned(),
        config: reset::__get_config(),
        compare_by_id: reset::__compare_by_id(),
        library_benchmarks: ::alloc::vec::Vec::new(),
        has_setup: reset::__run_setup(false),
        has_teardown: reset::__run_teardown(false),
    };
    for (function_name, get_config, macro_lib_benches) in reset::__BENCHES {
        let mut benches = ::iai_callgrind::__internal::InternalLibraryBenchmarkBenches {
            benches: ::alloc::vec::Vec::new(),
            config: get_config(),
        };
        for macro_lib_bench in macro_lib_benches.iter() {
            let bench = ::iai_callgrind::__internal::InternalLibraryBenchmarkBench {
                id: macro_lib_bench.id_display.map(|i| i.to_string()),
                args: macro_lib_bench.args_display.map(|i| i.to_string()),
                function_name: function_name.to_string(),
                config: macro_lib_bench.config.map(|f| f()),
            };
            benches.benches.push(bench);
        }
        internal_group.library_benchmarks.push(benches);
    }
    internal_benchmark_groups.groups.push(internal_group);
    let encoded = ::iai_callgrind::bincode::serialize(&internal_benchmark_groups)
        .expect("Encoded benchmark");
    if let Err(errors) = runner.exec(encoded) {
        {
            ::std::io::_eprint(format_args!("{0}\n", errors));
        };
        std::process::exit(1);
    }
}
#[inline(never)]
fn __run_setup(__run: bool) -> bool {
    let mut __has_setup = false;
    __has_setup
}
#[inline(never)]
fn __run_teardown(__run: bool) -> bool {
    let mut __has_teardown = false;
    __has_teardown
}
fn main() {
    let mut args_iter = std::hint::black_box(std::env::args()).skip(1);
    if args_iter.next().as_ref().map_or(false, |value| value == "--iai-run") {
        let current = std::hint::black_box(
            args_iter.next().expect("Expecting a function type"),
        );
        let next = std::hint::black_box(args_iter.next());
        match current.as_str() {
            "setup" if next.is_none() => {
                __run_setup(true);
            }
            "teardown" if next.is_none() => {
                __run_teardown(true);
            }
            "alloc_u8" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        alloc_u8::__run_setup(true);
                    }
                    "teardown" => {
                        alloc_u8::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        alloc_u8::__run(group_index, bench_index);
                    }
                }
            }
            "alloc_u32" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        alloc_u32::__run_setup(true);
                    }
                    "teardown" => {
                        alloc_u32::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        alloc_u32::__run(group_index, bench_index);
                    }
                }
            }
            "alloc_u32_aligned" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        alloc_u32_aligned::__run_setup(true);
                    }
                    "teardown" => {
                        alloc_u32_aligned::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        alloc_u32_aligned::__run(group_index, bench_index);
                    }
                }
            }
            "try_alloc_u32" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        try_alloc_u32::__run_setup(true);
                    }
                    "teardown" => {
                        try_alloc_u32::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        try_alloc_u32::__run(group_index, bench_index);
                    }
                }
            }
            "try_alloc_u32_aligned" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        try_alloc_u32_aligned::__run_setup(true);
                    }
                    "teardown" => {
                        try_alloc_u32_aligned::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        try_alloc_u32_aligned::__run(group_index, bench_index);
                    }
                }
            }
            "allocate" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        allocate::__run_setup(true);
                    }
                    "teardown" => {
                        allocate::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        allocate::__run(group_index, bench_index);
                    }
                }
            }
            "grow_same_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        grow_same_align::__run_setup(true);
                    }
                    "teardown" => {
                        grow_same_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        grow_same_align::__run(group_index, bench_index);
                    }
                }
            }
            "grow_smaller_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        grow_smaller_align::__run_setup(true);
                    }
                    "teardown" => {
                        grow_smaller_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        grow_smaller_align::__run(group_index, bench_index);
                    }
                }
            }
            "grow_larger_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        grow_larger_align::__run_setup(true);
                    }
                    "teardown" => {
                        grow_larger_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        grow_larger_align::__run(group_index, bench_index);
                    }
                }
            }
            "shrink_same_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        shrink_same_align::__run_setup(true);
                    }
                    "teardown" => {
                        shrink_same_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        shrink_same_align::__run(group_index, bench_index);
                    }
                }
            }
            "shrink_smaller_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        shrink_smaller_align::__run_setup(true);
                    }
                    "teardown" => {
                        shrink_smaller_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        shrink_smaller_align::__run(group_index, bench_index);
                    }
                }
            }
            "shrink_larger_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        shrink_larger_align::__run_setup(true);
                    }
                    "teardown" => {
                        shrink_larger_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        shrink_larger_align::__run(group_index, bench_index);
                    }
                }
            }
            "deallocate" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        deallocate::__run_setup(true);
                    }
                    "teardown" => {
                        deallocate::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        deallocate::__run(group_index, bench_index);
                    }
                }
            }
            "deallocate_non_last" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        deallocate_non_last::__run_setup(true);
                    }
                    "teardown" => {
                        deallocate_non_last::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        deallocate_non_last::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_allocate" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_allocate::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_allocate::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_allocate::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_grow_same_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_grow_same_align::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_grow_same_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_grow_same_align::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_grow_smaller_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_grow_smaller_align::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_grow_smaller_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_grow_smaller_align::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_grow_larger_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_grow_larger_align::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_grow_larger_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_grow_larger_align::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_shrink_same_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_shrink_same_align::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_shrink_same_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_shrink_same_align::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_shrink_smaller_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_shrink_smaller_align::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_shrink_smaller_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_shrink_smaller_align::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_shrink_larger_align" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_shrink_larger_align::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_shrink_larger_align::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_shrink_larger_align::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_deallocate" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_deallocate::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_deallocate::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_deallocate::__run(group_index, bench_index);
                    }
                }
            }
            "black_box_deallocate_non_last" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        black_box_deallocate_non_last::__run_setup(true);
                    }
                    "teardown" => {
                        black_box_deallocate_non_last::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        black_box_deallocate_non_last::__run(group_index, bench_index);
                    }
                }
            }
            "warm_up" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        warm_up::__run_setup(true);
                    }
                    "teardown" => {
                        warm_up::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        warm_up::__run(group_index, bench_index);
                    }
                }
            }
            "reset" => {
                match std::hint::black_box(
                    next
                        .expect(
                            "An argument `setup`, `teardown` or an index should be present",
                        )
                        .as_str(),
                ) {
                    "setup" => {
                        reset::__run_setup(true);
                    }
                    "teardown" => {
                        reset::__run_teardown(true);
                    }
                    value => {
                        let group_index = std::hint::black_box(
                            value
                                .parse::<usize>()
                                .expect("Expecting a valid group index"),
                        );
                        let bench_index = std::hint::black_box(
                            args_iter
                                .next()
                                .expect("A bench index should be present")
                                .parse::<usize>()
                                .expect("Expecting a valid bench index"),
                        );
                        reset::__run(group_index, bench_index);
                    }
                }
            }
            name => {
                ::core::panicking::panic_fmt(
                    format_args!("function \'{0}\' not found in this scope", name),
                );
            }
        }
    } else {
        std::hint::black_box(__run());
    };
}
