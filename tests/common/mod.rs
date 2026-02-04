#![cfg(all(feature = "std", feature = "panic-on-alloc"))]
#![allow(unused)]

mod instrumented_allocator;
mod limited_allocator;
mod test_wrap;

use std::{
    alloc::{Layout, System},
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
    Bump, BumpBox, BumpScope, BumpString, BumpVec, MutBumpString, MutBumpVec, MutBumpVecRev,
    alloc::{AllocError, Allocator, Global},
    mut_bump_format, mut_bump_vec, mut_bump_vec_rev, owned_slice,
    settings::BumpSettings,
    stats::Chunk,
    traits::{BumpAllocator, BumpAllocatorTyped as _},
};

pub(crate) use instrumented_allocator::InstrumentedAllocator;
pub(crate) use limited_allocator::Limited;
pub(crate) use test_wrap::TestWrap;

pub(crate) type AssumedMallocOverhead = [usize; 2];
type Result<T = (), E = AllocError> = core::result::Result<T, E>;

#[repr(C, align(16))]
pub(crate) struct ChunkHeader {
    pub(crate) pos: Cell<NonNull<u8>>,
    pub(crate) end: NonNull<u8>,

    pub(crate) prev: Cell<Option<NonNull<Self>>>,
    pub(crate) next: Cell<Option<NonNull<Self>>>,
}

pub(crate) const MALLOC_OVERHEAD: usize = size_of::<AssumedMallocOverhead>();
pub(crate) const OVERHEAD: usize = MALLOC_OVERHEAD + size_of::<ChunkHeader>();

pub(crate) type SettingNoMinSize<const UP: bool> = BumpSettings<1, UP, false, true, true, true, 0>;
pub(crate) type BumpNoMinSize<const UP: bool, A = Global> = bump_scope::Bump<A, SettingNoMinSize<UP>>;
pub(crate) type BumpScopeNoMinSize<'a, const UP: bool, A = Global> = bump_scope::BumpScope<'a, A, SettingNoMinSize<UP>>;

macro_rules! either_way {
    ($($(#[$attr:meta])* $ident:ident)*) => {
        mod up {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `true`");
                    super::$ident::<true>();
                }
            )*
        }

        mod down {
            $(
                #[test]
                $(#[$attr])*
                fn $ident() {
                    std::eprintln!("`UP` is `false`");
                    super::$ident::<false>();
                }
            )*
        }
    };
}

pub(crate) use either_way;

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
