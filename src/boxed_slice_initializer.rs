use core::{
    alloc::Allocator,
    mem::{MaybeUninit, forget},
    ptr,
};

use alloc::boxed::Box;

use crate::{SizedTypeProperties, destructure::destructure};

/// Allows for initializing a `BoxU<[MaybeUninit<T>]>` by pushing values.
/// On drop, this will drop the values that have been pushed so far.
pub(crate) struct BoxedSliceInitializer<T, A: Allocator> {
    pos: *mut T,
    end: *mut T, // if T is a ZST this is ptr + len

    boxed: Box<[MaybeUninit<T>], A>,
}

impl<T, A: Allocator> Drop for BoxedSliceInitializer<T, A> {
    fn drop(&mut self) {
        unsafe {
            let to_drop_len = self.init_len();
            let to_drop = ptr::slice_from_raw_parts_mut(self.start_mut(), to_drop_len);
            to_drop.drop_in_place();
        }
    }
}

impl<T, A: Allocator> BoxedSliceInitializer<T, A> {
    #[inline(always)]
    pub(crate) fn new(mut boxed: Box<[MaybeUninit<T>], A>) -> Self {
        let len = boxed.len();
        let pos = boxed.as_mut_ptr().cast::<T>();

        if T::IS_ZST {
            let end = pos.wrapping_byte_add(len);
            return Self { pos, end, boxed };
        }

        Self {
            pos,
            end: unsafe { pos.add(len) },
            boxed,
        }
    }

    #[inline(always)]
    fn start(&self) -> *const T {
        self.boxed.as_ptr().cast()
    }

    #[inline(always)]
    fn start_mut(&mut self) -> *mut T {
        self.boxed.as_mut_ptr().cast()
    }

    #[inline(always)]
    fn init_len(&self) -> usize {
        if T::IS_ZST {
            self.pos.addr().wrapping_sub(self.start().addr())
        } else {
            unsafe { self.pos.cast_const().offset_from_unsigned(self.start()) }
        }
    }

    #[inline(always)]
    #[expect(unused)]
    fn len(&self) -> usize {
        if T::IS_ZST {
            self.end.addr().wrapping_sub(self.start().addr())
        } else {
            unsafe { self.end.offset_from_unsigned(self.start()) }
        }
    }

    #[inline(always)]
    pub(crate) fn is_full(&self) -> bool {
        self.pos == self.end
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, value: T) {
        self.push_with(|| value);
    }

    #[inline(always)]
    pub(crate) fn push_with(&mut self, f: impl FnOnce() -> T) {
        assert!(!self.is_full());
        unsafe { self.push_unchecked(f()) }
    }

    #[inline(always)]
    pub(crate) unsafe fn push_unchecked(&mut self, value: T) {
        debug_assert!(!self.is_full());

        unsafe {
            if T::IS_ZST {
                forget(value);
                self.pos = self.pos.wrapping_byte_add(1);
            } else {
                self.pos.write(value);
                self.pos = self.pos.add(1);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn into_init(self) -> Box<[T], A> {
        assert!(self.is_full());
        unsafe { self.into_init_unchecked() }
    }

    #[inline(always)]
    pub(crate) unsafe fn into_init_unchecked(self) -> Box<[T], A> {
        debug_assert!(self.is_full());
        destructure!(let Self { boxed } = self);
        unsafe { Box::<[MaybeUninit<T>], A>::assume_init(boxed) }
    }
}
