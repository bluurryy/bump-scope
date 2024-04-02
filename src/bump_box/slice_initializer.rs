use core::{
    marker::PhantomData,
    mem::{ManuallyDrop, MaybeUninit},
    ptr::NonNull,
};

use crate::{
    polyfill::{nonnull, pointer},
    BumpBox, SizedTypeProperties,
};

/// Allows for initializing a `BumpBox<[MaybeUninit<T>]>` by pushing values.
/// On drop, this will drop the values that have been pushed so far.
pub(crate) struct BumpBoxSliceInitializer<'a, T> {
    pos: NonNull<T>,

    start: NonNull<T>,
    end: NonNull<T>, // if T is a ZST this is ptr + len

    /// First field marks the lifetime.
    /// Second field marks ownership over T. (<https://doc.rust-lang.org/nomicon/phantom-data.html#generic-parameters-and-drop-checking>)
    marker: PhantomData<(&'a (), T)>,
}

impl<T> Drop for BumpBoxSliceInitializer<'_, T> {
    fn drop(&mut self) {
        unsafe {
            let to_drop_len = self.init_len();
            let to_drop = nonnull::slice_from_raw_parts(self.start, to_drop_len);
            to_drop.as_ptr().drop_in_place();
        }
    }
}

impl<'a, T> BumpBoxSliceInitializer<'a, T> {
    #[inline(always)]
    pub fn new(slice: BumpBox<'a, [MaybeUninit<T>]>) -> Self {
        if T::IS_ZST {
            return Self {
                pos: NonNull::dangling(),

                start: NonNull::dangling(),
                end: unsafe { nonnull::wrapping_byte_add(NonNull::dangling(), slice.len()) },

                marker: PhantomData,
            };
        }

        let len = slice.len();
        let slice = slice.into_raw();

        unsafe {
            let start = slice.cast::<T>();
            let end = nonnull::add(start, len);

            Self {
                pos: start,
                start,
                end,
                marker: PhantomData,
            }
        }
    }

    #[inline(always)]
    fn init_len(&self) -> usize {
        if T::IS_ZST {
            nonnull::addr(self.pos).get().wrapping_sub(nonnull::addr(self.start).get())
        } else {
            unsafe { nonnull::sub_ptr(self.pos, self.start) }
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        if T::IS_ZST {
            nonnull::addr(self.end).get().wrapping_sub(nonnull::addr(self.start).get())
        } else {
            unsafe { nonnull::sub_ptr(self.end, self.start) }
        }
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool {
        self.pos == self.end
    }

    #[inline(always)]
    pub fn push(&mut self, value: T) {
        self.push_with(|| value);
    }

    #[inline(always)]
    pub fn push_with(&mut self, f: impl FnOnce() -> T) {
        assert!(!self.is_full());
        unsafe { self.push_with_unchecked(f) }
    }

    #[inline(always)]
    pub unsafe fn push_unchecked(&mut self, value: T) {
        self.push_with_unchecked(|| value);
    }

    #[inline(always)]
    pub unsafe fn push_with_unchecked(&mut self, f: impl FnOnce() -> T) {
        debug_assert!(!self.is_full());
        pointer::write_with(self.pos.as_ptr(), f);

        if T::IS_ZST {
            self.pos = nonnull::wrapping_byte_add(self.pos, 1);
        } else {
            self.pos = nonnull::add(self.pos, 1);
        }
    }

    #[inline(always)]
    pub fn into_init(self) -> BumpBox<'a, [T]> {
        assert!(self.is_full());
        unsafe { self.into_init_unchecked() }
    }

    #[inline(always)]
    pub unsafe fn into_init_unchecked(self) -> BumpBox<'a, [T]> {
        let this = ManuallyDrop::new(self);
        debug_assert!(this.is_full());
        let len = this.len();
        let slice = nonnull::slice_from_raw_parts(this.start, len);
        BumpBox::from_raw(slice)
    }
}
