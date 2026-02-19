use core::{
    marker::PhantomData,
    mem::{self, ManuallyDrop, MaybeUninit},
    ptr::{self, NonNull},
};

use crate::{BumpBox, SizedTypeProperties, polyfill::non_null};

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
            let to_drop = NonNull::slice_from_raw_parts(self.start, to_drop_len);
            to_drop.drop_in_place();
        }
    }
}

impl<'a, T> BumpBoxSliceInitializer<'a, T> {
    #[inline(always)]
    pub(crate) fn new(slice: BumpBox<'a, [MaybeUninit<T>]>) -> Self {
        if T::IS_ZST {
            let start = NonNull::dangling();
            let end = unsafe { non_null::wrapping_byte_add(start, slice.len()) };

            return Self {
                pos: start,
                start,
                end,
                marker: PhantomData,
            };
        }

        let len = slice.len();
        let slice = slice.into_raw();

        unsafe {
            let start = slice.cast::<T>();
            let end = start.add(len);

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
            self.pos.addr().get().wrapping_sub(self.start.addr().get())
        } else {
            unsafe { non_null::offset_from_unsigned(self.pos, self.start) }
        }
    }

    #[inline(always)]
    fn len(&self) -> usize {
        if T::IS_ZST {
            self.end.addr().get().wrapping_sub(self.start.addr().get())
        } else {
            unsafe { non_null::offset_from_unsigned(self.end, self.start) }
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
                mem::forget(value);
                self.pos = non_null::wrapping_byte_add(self.pos, 1);
            } else {
                ptr::write(self.pos.as_ptr(), value);
                self.pos = self.pos.add(1);
            }
        }
    }

    #[inline(always)]
    pub(crate) fn into_init(self) -> BumpBox<'a, [T]> {
        assert!(self.is_full());
        unsafe { self.into_init_unchecked() }
    }

    #[inline(always)]
    pub(crate) unsafe fn into_init_unchecked(self) -> BumpBox<'a, [T]> {
        unsafe {
            let this = ManuallyDrop::new(self);
            debug_assert!(this.is_full());
            let len = this.len();
            let slice = NonNull::slice_from_raw_parts(this.start, len);
            BumpBox::from_raw(slice)
        }
    }
}
