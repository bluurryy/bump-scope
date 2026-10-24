#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![allow(unused)]

mod disaligning_allocator;
mod expect_leaks;
mod instrumented_allocator;
mod limited_allocator;
mod test_wrap;

use std::{
    alloc::{AllocError, Allocator, Global, Layout, System},
    any::Any,
    boxed::Box,
    cell::Cell,
    dbg, eprintln,
    fmt::Debug,
    io::IoSlice,
    iter, mem,
    ops::Index,
    ptr::NonNull,
    string::{String, ToString},
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
    thread_local,
    vec::Vec,
};

use bump_scope::{
    BoxU, Bump, BumpScope,
    config::{BumpConfig, BumpConfigBuilder},
    stats::Chunk,
    traits::BumpAllocator as _,
};

pub(crate) use disaligning_allocator::DisaligningAllocator;
pub(crate) use expect_leaks::ExpectLeaks;
pub(crate) use instrumented_allocator::InstrumentedAllocator;
pub(crate) use limited_allocator::Limited;
pub(crate) use test_wrap::TestWrap;

pub(crate) type AssumedMallocOverhead = [usize; 2];
type Result<T = (), E = AllocError> = core::result::Result<T, E>;

pub(crate) const MALLOC_OVERHEAD: usize = size_of::<AssumedMallocOverhead>();

macro_rules! up_and_down {
    ($($tt:tt)*) => {
        mod upwards {
            #[allow(unused_imports)]
            use super::*;

            #[allow(unused_imports)]
            use bump_scope::{
                config::{BumpConfig, BumpConfigBuilder},
                BumpAllocator, BumpAllocatorScope, extra, vec
            };

            const DEFAULT_C: BumpConfig = BumpConfigBuilder::new()
                .upwards(true)
                .build();

            $($tt)*
        }

        mod downwards {
            #[allow(unused_imports)]
            use super::*;

            #[allow(unused_imports)]
            use bump_scope::{
                config::{BumpConfig, BumpConfigBuilder},
                BumpAllocator, BumpAllocatorScope, extra, vec
            };

            const DEFAULT_C: BumpConfig = BumpConfigBuilder::new()
                .upwards(false)
                .build();

            $($tt)*
        }
    };
}

pub(crate) use up_and_down;

pub(crate) fn expect_no_panic<T>(result: Result<T, Box<dyn Any + Send>>) -> T {
    match result {
        Ok(value) => value,
        Err(payload) => unexpected_panic(payload),
    }
}

#[expect(clippy::match_wild_err_arm)]
fn unexpected_panic(payload: Box<dyn Any + Send>) -> ! {
    match panic_payload_string(payload) {
        Ok(msg) => panic!("unexpected panic: {msg}"),
        Err(_) => panic!("unexpected panic (no message)"),
    }
}

fn panic_payload_string(payload: Box<dyn Any + Send>) -> Result<String, Box<dyn Any + Send>> {
    let payload = match payload.downcast::<&'static str>() {
        Ok(string) => return Ok(string.to_string()),
        Err(payload) => payload,
    };

    let payload = match payload.downcast::<String>() {
        Ok(string) => return Ok(*string),
        Err(payload) => payload,
    };

    Err(payload)
}
