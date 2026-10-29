#[cfg(feature = "nightly")]
use core::clone::CloneToUninit;
use core::{
    alloc::{AllocError, Allocator, Layout},
    ffi::CStr,
    fmt,
    mem::MaybeUninit,
    num::NonZeroUsize,
    ptr::{self, NonNull},
};

#[cfg(feature = "nightly")]
use alloc::alloc::AllocatorNightly;
use alloc::{boxed::Box, vec::Vec};

use crate::{
    Bump, BumpAllocation, BumpScope, Checkpoint, SizedTypeProperties, Unmanaged,
    bump_allocation::IntoRef,
    bump_down,
    config::{BumpConfig, DEFAULT_BUMP_CONFIG, config},
    down_align_usize,
    error_behavior::ErrorBehavior,
    extra::{
        self, box_from_utf8_unchecked, box_init, box_init_clone, box_init_fill, box_init_fill_with,
        box_replace_allocator_unchecked, box_uninit_zst_in, box_uninit_zst_slice_in, imp_vec_extend,
        vec_remove_allocator_unchecked,
    },
    owned_slice::{OwnedSlice, TakeOwnedSlice as _},
    polyfill::{transmute_mut, transmute_ref},
    stats::AnyStats,
    string::String,
    up_align_usize_unchecked, util,
};

#[cfg(feature = "panic-on-alloc")]
use crate::{error_behavior, panic_on_error};

#[cfg(feature = "nightly")]
impl<A: Allocator, const C: BumpConfig> AllocatorNightly for Bump<A, C> {}
#[cfg(feature = "nightly")]
impl<A: Allocator, const C: BumpConfig> AllocatorNightly for BumpScope<'_, A, C> {}
#[cfg(feature = "nightly")]
impl AllocatorNightly for Unmanaged<'_> {}

/// TODO
///
/// # Safety
/// TODO
#[allow(unused)]
pub(crate) unsafe trait Sealed: Allocator + Sized {
    const CONFIG: BumpConfig;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn(&self, layout: Layout) -> Result<NonNull<u8>, AllocError> {
    /// Ok(self.allocate(layout)?.cast::<u8>())
    /// #     }
    /// # }
    /// ```
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::new::<T>())?.cast::<T>())
    /// #     }
    /// # }
    /// ```
    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///    
    /// Behaves like:
    /// ```
    /// # extern crate alloc;
    /// # use core::{alloc::Layout, alloc::Allocator, ptr::NonNull, error::Error};
    /// # use alloc::boxed::Box;
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self, len: usize) -> Result<NonNull<T>, Box<dyn Error>> {
    /// Ok(self.allocate(Layout::array::<T>(len)?)?.cast::<T>())
    /// #     }
    /// # }
    /// ```
    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// Behaves like:
    /// ```
    /// # use core::{alloc::Layout, alloc::{Allocator, AllocError}, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self, slice: &[T]) -> Result<NonNull<T>, AllocError> {
    /// Ok(self.allocate(Layout::for_value(slice))?.cast::<T>())
    /// #     }
    /// # }
    /// ```
    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E>;

    /// A specialized version of [`allocate`](core::alloc::Allocator::allocate).
    ///
    /// The allocation takes up as much of the free space of the chunk as possible.
    fn spec_alloc_slice_greedy<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E>;

    /// A specialized version of [`shrink`](core::alloc::Allocator::shrink).
    ///
    /// Behaves like the following code except that it returns `None`
    /// when the allocation remains unchanged and the pointer stays valid.
    /// ```
    /// # use core::{alloc::Layout, alloc::Allocator, ptr::NonNull};
    /// # #[expect(dead_code)]
    /// # trait MyExt: Allocator {
    /// #     unsafe fn my_ext_fn<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> NonNull<T> {
    /// self.shrink(ptr.cast(),
    ///     Layout::array::<T>(old_len).unwrap_unchecked(),
    ///     Layout::array::<T>(new_len).unwrap_unchecked(),
    /// ).unwrap_unchecked().cast()
    /// #     }
    /// # }
    /// ```
    ///
    /// # Safety
    ///
    /// Same safety conditions as for the code above apply.
    ///
    /// [shrink]: core::alloc::Allocator::shrink
    /// [array]: Layout::array
    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>>;

    /// TODO
    fn spec_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B>;

    /// A specialized version of [`prepare_allocation`].
    ///
    /// Returns a `[T]` of free space in the bump allocator.
    ///
    /// [`prepare_allocation`]: crate::traits::BumpAllocatorCore::prepare_allocation
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E>;

    /// A specialized version of [`allocate_prepared`].
    ///
    /// Allocates part of the free space returned from a
    /// <code>([try_](BumpAllocator::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocator::prepare_slice_allocation)</code>
    /// call.
    ///
    /// # Safety
    /// - `ptr..ptr + cap` must be the pointer range returned from
    ///   <code>([try_](BumpAllocator::try_prepare_slice_allocation))[prepare_slice_allocation](BumpAllocator::prepare_slice_allocation)</code>.
    /// - no allocation, grow, shrink or deallocate must have taken place since then
    /// - no resets must have taken place since then
    /// - `len` must be less than or equal to `cap`
    ///
    /// [`allocate_prepared`]: BumpAllocatorCore::allocate_prepared
    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]>;

    /// See [`BumpAllocator::checkpoint`].
    #[must_use]
    fn checkpoint(&self) -> Checkpoint;

    /// See [`BumpAllocator::reset_to`].
    unsafe fn reset_to(&self, checkpoint: Checkpoint);

    /// TODO
    fn pos(&self) -> *mut u8;

    /// TODO
    ///
    /// # Safety
    ///
    /// Chunk must be allocated.
    unsafe fn set_pos(&self, addr: usize);

    #[inline(always)]
    fn imp_alloc<E, T>(&self, value: T) -> Result<Box<T, &Self>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init(self.imp_alloc_uninit()?, value))
    }

    #[inline(always)]
    fn imp_alloc_with<E, T>(&self, f: impl FnOnce() -> T) -> Result<Box<T, &Self>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init(self.imp_alloc_uninit()?, f()))
    }

    #[inline(always)]
    fn imp_alloc_default<E, T>(&self) -> Result<Box<T, &Self>, E>
    where
        E: ErrorBehavior,
        T: Default,
    {
        self.imp_alloc_with(T::default)
    }

    #[inline(always)]
    #[cfg(feature = "nightly")]
    fn imp_alloc_clone<E, T>(&self, value: &T) -> Result<Box<T, &Self>, E>
    where
        E: ErrorBehavior,
        T: CloneToUninit + ?Sized,
    {
        let data = self.spec_alloc(Layout::for_value(value))?;
        let metadata = ptr::metadata(value);

        unsafe {
            value.clone_to_uninit(data.as_ptr());
            let ptr = ptr::from_raw_parts_mut(data.as_ptr(), metadata);
            Ok(Box::from_raw_in(ptr, self))
        }
    }

    #[inline(always)]
    fn imp_alloc_uninit<E, T>(&self) -> Result<Box<MaybeUninit<T>, &Self>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            return unsafe { Ok(Box::from_raw_in(ptr::dangling_mut(), self)) };
        }

        let ptr = self.spec_alloc_sized()?.as_ptr();
        unsafe { Ok(Box::from_raw_in(ptr, self)) }
    }

    #[inline(always)]
    fn imp_alloc_slice_move<E, T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
    {
        unsafe {
            let mut owned_slice = slice.into_take_owned_slice();
            let owned_slice_ref = owned_slice.owned_slice_ref();

            let len = owned_slice_ref.len();
            let src = owned_slice_ref.as_ptr();
            let dst = self.spec_alloc(Layout::for_value(owned_slice_ref))?.as_ptr().cast();
            ptr::copy_nonoverlapping(src, dst, len);

            owned_slice.take_owned_slice();
            Ok(Box::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), self))
        }
    }

    #[inline(always)]
    fn imp_alloc_slice_copy<E, T>(&self, slice: &[T]) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
        T: Copy,
    {
        if T::IS_ZST {
            return Ok(box_init_clone(box_uninit_zst_slice_in(slice.len(), self), slice));
        }

        let len = slice.len();
        let src = slice.as_ptr();
        let dst = self.spec_alloc_slice_for(slice)?.as_ptr();

        unsafe {
            core::ptr::copy_nonoverlapping(src, dst, len);
            Ok(Box::from_raw_in(ptr::slice_from_raw_parts_mut(dst, len), self))
        }
    }

    #[inline(always)]
    fn imp_alloc_slice_clone<E, T>(&self, slice: &[T]) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
        T: Clone,
    {
        Ok(box_init_clone(self.imp_alloc_uninit_slice_for(slice)?, slice))
    }

    #[inline(always)]
    fn imp_alloc_slice_fill<E, T>(&self, len: usize, value: T) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
        T: Clone,
    {
        Ok(box_init_fill(self.imp_alloc_uninit_slice(len)?, value))
    }

    #[inline(always)]
    fn imp_alloc_slice_fill_with<E, T>(&self, len: usize, f: impl FnMut() -> T) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
    {
        Ok(box_init_fill_with(self.imp_alloc_uninit_slice(len)?, f))
    }

    #[inline(always)]
    fn imp_alloc_uninit_slice<E, T>(&self, len: usize) -> Result<Box<[MaybeUninit<T>], &Self>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            return Ok(box_uninit_zst_slice_in(len, self));
        }

        let ptr = self.spec_alloc_slice::<E, MaybeUninit<T>>(len)?.as_ptr();

        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, len);
            Ok(Box::from_raw_in(slice_ptr, self))
        }
    }

    #[inline(always)]
    fn imp_alloc_uninit_slice_for<E, T>(&self, slice: &[T]) -> Result<Box<[MaybeUninit<T>], &Self>, E>
    where
        E: ErrorBehavior,
    {
        let ptr = self.spec_alloc_slice_for(slice)?.cast::<MaybeUninit<T>>().as_ptr();

        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, slice.len());
            Ok(Box::from_raw_in(slice_ptr, self))
        }
    }

    #[inline(always)]
    fn imp_alloc_str<E>(&self, src: &str) -> Result<Box<str, &Self>, E>
    where
        E: ErrorBehavior,
    {
        let bytes = self.imp_alloc_slice_copy(src.as_bytes())?;

        // SAFETY: src is utf8 so this is too
        Ok(unsafe { box_from_utf8_unchecked(bytes) })
    }

    #[inline(always)]
    fn imp_alloc_fmt<E>(&self, args: fmt::Arguments) -> Result<Box<str, &Self>, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_str(string);
        }

        let mut string = String::<_, E>::new(self);
        string.write_fmt(args)?;
        Ok(string.into_str())
    }

    #[inline(always)]
    fn imp_alloc_cstr<E>(&self, src: &CStr) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorBehavior,
    {
        let (slice, _) = Box::into_raw_with_allocator(self.imp_alloc_slice_copy(src.to_bytes_with_nul())?);

        // SAFETY: input is `CStr` so this is too
        let ptr = unsafe { CStr::from_bytes_with_nul_unchecked(&*slice) };

        Ok(unsafe { IntoRef::new(ptr, self) })
    }

    #[inline(always)]
    fn imp_alloc_cstr_from_str<E>(&self, src: &str) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorBehavior,
    {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.imp_alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.spec_alloc_slice::<E, u8>(src.len() + 1)?;

            unsafe {
                core::ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = core::slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                let ptr = CStr::from_bytes_with_nul_unchecked(bytes);

                Ok(IntoRef::new(ptr, self))
            }
        }
    }

    #[inline(always)]
    fn imp_alloc_cstr_fmt<E>(&self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return self.imp_alloc_cstr_from_str(string);
        }

        let mut string = String::<_, E>::new(self);
        string.write_fmt(args)?;
        string.into_cstr()
    }

    #[inline(always)]
    fn imp_alloc_iter<E, T>(&self, iter: impl IntoIterator<Item = T>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
    {
        let mut vec = Vec::new_in(self);
        imp_vec_extend(&mut vec, iter)?;
        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    fn imp_alloc_iter_exact<E, T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
        I: ExactSizeIterator<Item = T>,
    {
        let vec = extra::imp_vec_from_iter_exact_in(iter, self)?;

        // An exact size iterator if implemented correctly, did not leave anything to shrink
        // so we avoid the shrink call by removing the allocator.
        let vec = unsafe { vec_remove_allocator_unchecked(vec).0 };
        let boxed = vec.into_boxed_slice();
        let boxed = unsafe { box_replace_allocator_unchecked(boxed, self).0 };

        Ok(boxed)
    }

    #[inline(always)]
    fn imp_alloc_try_with<B, T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<Box<T, &Self>, E>, B>
    where
        B: ErrorBehavior,
    {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(box_init(box_uninit_zst_in(self), value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let checkpoint_before_alloc = self.checkpoint();
        let uninit = self.imp_alloc_uninit::<B, Result<T, E>>()?;
        let ptr = Box::into_raw_with_allocator(uninit).0.cast::<Result<T, E>>();

        // When bumping downwards the chunk's position is the same as `ptr`.
        // Using `ptr` is faster so we use that.
        let pos = if config!(Self::CONFIG, upwards) {
            self.pos()
        } else {
            ptr.cast::<u8>()
        };

        Ok(unsafe {
            ptr.write(f());

            match util::ptr_result(ptr) {
                Ok(value) => Ok(Box::from_raw_in(value, self)),
                Err(error) => Err({
                    let error = error.read();

                    // If `f` made allocations on this bump allocator we can't shrink the allocation.
                    let can_shrink = pos == self.pos();

                    if can_shrink {
                        self.reset_to(checkpoint_before_alloc);
                    }

                    error
                }),
            }
        })
    }

    /// TODO
    // FIXME: non-greedy variant for a more regular api / easy switching?
    fn imp_alloc_vec_greedy<E, T>(&self, len: usize) -> Result<Vec<T, &Self>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            return Ok(Vec::new_in(self));
        }

        let slice = self.spec_alloc_slice_greedy::<E, T>(len)?;
        let cap = slice.len();
        let ptr = slice.cast::<T>().as_ptr();
        unsafe { Ok(Vec::from_raw_parts_in(ptr, 0, cap, self)) }
    }

    #[inline(always)]
    fn imp_alloc_iter_mut<E, T>(&mut self, iter: impl IntoIterator<Item = T>) -> Result<Box<[T], &Self>, E>
    where
        E: ErrorBehavior,
    {
        if T::IS_ZST {
            let mut vec = Vec::new_in(&*self);
            vec.extend(iter);
            return Ok(vec.into_boxed_slice());
        }

        let mut vec = (*self).imp_alloc_vec_greedy::<E, T>(0)?;

        if E::FALLIBLE {
            for item in iter {
                if vec.len() == vec.capacity() && vec.try_reserve(1).is_err() {
                    return Err(E::allocation(Layout::new::<()>()));
                }
                vec.push(item);
            }
        } else {
            vec.extend(iter);
        }

        Ok(vec.into_boxed_slice())
    }

    #[inline(always)]
    fn imp_alloc_fmt_mut<E>(&mut self, args: fmt::Arguments) -> Result<Box<str, &Self>, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return (*self).imp_alloc_str(string);
        }

        let vec = (*self).imp_alloc_vec_greedy::<E, u8>(0)?;
        let mut string = String::<_, E>::new_from_vec(vec);
        string.write_fmt(args)?;
        Ok(string.into_str())
    }

    #[inline(always)]
    fn imp_alloc_cstr_fmt_mut<E>(&mut self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, E>
    where
        E: ErrorBehavior,
    {
        if let Some(string) = args.as_str() {
            return (*self).imp_alloc_cstr_from_str(string);
        }

        let vec = (*self).imp_alloc_vec_greedy(0)?;
        let mut string = String::new_from_vec(vec);
        string.write_fmt(args)?;
        string.into_cstr()
    }

    #[inline(always)]
    fn imp_alloc_try_with_mut<B, T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<Box<T, &Self>, E>, B>
    where
        B: ErrorBehavior,
    {
        if T::IS_ZST {
            return match f() {
                Ok(value) => Ok(Ok(box_init(box_uninit_zst_in(self), value))),
                Err(error) => Ok(Err(error)),
            };
        }

        let ptr = (*self).spec_prepare_sized_allocation::<B, Result<T, E>>()?.as_ptr();

        Ok(unsafe {
            ptr.write(f());

            // There is no need for `can_shrink` checks, because we have a mutable reference
            // so there's no way anyone else has allocated in `f`.
            match util::ptr_result(ptr) {
                Ok(value) => Ok({
                    let new_pos = if config!(Self::CONFIG, upwards) {
                        let pos = value.add(1).addr();
                        up_align_usize_unchecked(pos, config!(Self::CONFIG, min_align))
                    } else {
                        let pos = value.addr();
                        down_align_usize(pos, config!(Self::CONFIG, min_align))
                    };

                    // SAFETY: The allocation was successful, so our chunk must be allocated.
                    self.set_pos(new_pos);

                    Box::from_raw_in(value, self)
                }),
                Err(error) => Err(error.read()),
            }
        })
    }
}
unsafe impl<A: Sealed> Sealed for &A {
    const CONFIG: BumpConfig = A::CONFIG;

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        A::checkpoint(self)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { A::reset_to(self, checkpoint) };
    }

    #[inline(always)]
    fn pos(&self) -> *mut u8 {
        A::pos(self)
    }

    #[inline(always)]
    unsafe fn set_pos(&self, addr: usize) {
        unsafe { A::set_pos(self, addr) }
    }

    #[inline(always)]
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        A::spec_alloc(self, layout)
    }

    #[inline(always)]
    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        A::spec_alloc_sized(self)
    }

    #[inline(always)]
    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice(self, len)
    }

    #[inline(always)]
    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice_for(self, slice)
    }

    #[inline(always)]
    fn spec_alloc_slice_greedy<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        A::spec_alloc_slice_greedy(self, len)
    }

    #[inline(always)]
    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { A::spec_shrink_slice(self, ptr, old_len, new_len) }
    }

    #[inline(always)]
    fn spec_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        A::spec_prepare_sized_allocation(self)
    }

    #[inline(always)]
    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        A::spec_prepare_slice_allocation(self, cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { A::spec_alloc_prepared_slice(self, start, len, cap) }
    }
}
unsafe impl<A: Sealed> Sealed for &mut A {
    const CONFIG: BumpConfig = A::CONFIG;

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        A::checkpoint(self)
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { A::reset_to(self, checkpoint) };
    }

    #[inline(always)]
    fn pos(&self) -> *mut u8 {
        A::pos(self)
    }

    #[inline(always)]
    unsafe fn set_pos(&self, addr: usize) {
        unsafe { A::set_pos(self, addr) }
    }

    #[inline(always)]
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        A::spec_alloc(self, layout)
    }

    #[inline(always)]
    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        A::spec_alloc_sized(self)
    }

    #[inline(always)]
    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice(self, len)
    }

    #[inline(always)]
    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        A::spec_alloc_slice_for(self, slice)
    }

    #[inline(always)]
    fn spec_alloc_slice_greedy<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        A::spec_alloc_slice_greedy(self, len)
    }

    #[inline(always)]
    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { A::spec_shrink_slice(self, ptr, old_len, new_len) }
    }

    #[inline(always)]
    fn spec_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        A::spec_prepare_sized_allocation(self)
    }

    #[inline(always)]
    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        A::spec_prepare_slice_allocation(self, cap)
    }

    #[inline(always)]
    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { A::spec_alloc_prepared_slice(self, start, len, cap) }
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> Sealed for Bump<A, C> {
    const CONFIG: BumpConfig = C;

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.raw.checkpoint()
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { self.raw.reset_to(checkpoint) };
    }

    #[inline(always)]
    fn pos(&self) -> *mut u8 {
        self.raw.chunk.get().pos().as_ptr()
    }

    #[inline(always)]
    unsafe fn set_pos(&self, addr: usize) {
        unsafe { self.raw.chunk.get().as_allocated_unchecked().set_pos_addr(addr) };
    }

    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        self.as_bump_scope().spec_alloc(layout)
    }

    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        self.as_bump_scope().spec_alloc_sized()
    }

    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        self.as_bump_scope().spec_alloc_slice(len)
    }

    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        self.as_bump_scope().spec_alloc_slice_for(slice)
    }

    fn spec_alloc_slice_greedy<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        self.as_bump_scope().spec_alloc_slice_greedy(len)
    }

    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        unsafe { self.as_bump_scope().spec_shrink_slice(ptr, old_len, new_len) }
    }

    #[inline(always)]
    fn spec_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        self.as_bump_scope().spec_prepare_sized_allocation()
    }

    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        self.as_bump_scope().spec_prepare_slice_allocation(cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe { self.as_bump_scope().spec_alloc_prepared_slice(start, len, cap) }
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> Sealed for BumpScope<'_, A, C> {
    const CONFIG: BumpConfig = C;

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        self.raw.checkpoint()
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { self.raw.reset_to(checkpoint) };
    }

    #[inline(always)]
    fn pos(&self) -> *mut u8 {
        self.raw.chunk.get().pos().as_ptr()
    }

    #[inline(always)]
    unsafe fn set_pos(&self, addr: usize) {
        unsafe { self.raw.chunk.get().as_allocated_unchecked().set_pos_addr(addr) };
    }

    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        self.raw.allocate(layout)
    }

    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        self.raw.allocate_sized()
    }

    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        self.raw.allocate_slice(len)
    }

    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        self.raw.allocate_slice_for(slice)
    }

    fn spec_alloc_slice_greedy<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        // FIXME: simplify `prepare` api / replace it with greedy slice alloc
        let slice = self.raw.prepare_slice_allocation(len)?;
        let len = slice.len();
        let ptr = slice.cast::<T>().as_ptr();

        unsafe {
            // a successful `prepare_allocation` guarantees a non-dummy-chunk
            let chunk = self.raw.chunk.get().as_allocated_unchecked();

            if config!(upwards) {
                chunk.set_pos_addr_and_align_from(ptr.add(len).addr(), T::ALIGN);
            } else {
                chunk.set_pos_addr_and_align_from(ptr.addr(), T::ALIGN);
            }
        }

        Ok(slice)
    }

    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        if !config!(shrinks) {
            return None;
        }

        let old_ptr = ptr.cast::<u8>();
        let old_size = old_len * T::SIZE; // we already allocated that amount so this can't overflow
        let new_size = new_len * T::SIZE; // its less than the capacity so this can't overflow

        // Adapted from `Allocator::shrink`.
        unsafe {
            let chunk = self.raw.chunk.get();

            let is_last = if config!(upwards) {
                old_ptr.as_ptr().add(old_size) == chunk.pos().as_ptr()
            } else {
                old_ptr == chunk.pos()
            };

            // if that's not the last allocation, there is nothing we can do
            if !is_last {
                return None;
            }

            // `is_last` is true, which guarantees a non-dummy, see `allocator_impl::is_last`
            let chunk = chunk.as_allocated_unchecked();

            if config!(upwards) {
                let end = old_ptr.addr().get() + new_size;

                // Up-aligning a pointer inside a chunk by `MIN_ALIGN` never overflows.
                let new_pos = up_align_usize_unchecked(end, config!(min_align));

                chunk.set_pos_addr(new_pos);
                Some(old_ptr.cast())
            } else {
                let old_addr = old_ptr.addr();
                let old_addr_old_end = NonZeroUsize::new_unchecked(old_addr.get() + old_size);

                let new_addr = bump_down(old_addr_old_end, new_size, T::ALIGN.max(config!(min_align)));
                let new_addr = NonZeroUsize::new_unchecked(new_addr);
                let old_addr_new_end = NonZeroUsize::new_unchecked(old_addr.get() + new_size);

                let new_ptr = old_ptr.with_addr(new_addr);
                let overlaps = old_addr_new_end > new_addr;

                if overlaps {
                    old_ptr.copy_to(new_ptr, new_size);
                } else {
                    old_ptr.copy_to_nonoverlapping(new_ptr, new_size);
                }

                chunk.set_pos(new_ptr);
                Some(new_ptr.cast())
            }
        }
    }

    #[inline(always)]
    fn spec_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        self.raw.prepare_sized_allocation()
    }

    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        self.raw.prepare_slice_allocation(cap)
    }

    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        unsafe {
            // a successful `prepare_allocation` guarantees a non-dummy-chunk
            let chunk = self.raw.chunk.get().as_allocated_unchecked();

            let end = start.add(len);

            if config!(Self::CONFIG, upwards) {
                chunk.set_pos_addr_and_align_from(end.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(start, len)
            } else {
                let dst_end = start.add(cap);
                let dst = dst_end.sub(len);
                start.copy_to(dst, len); // FIXME: conditional copy_nonoverlapping?
                chunk.set_pos_addr_and_align_from(dst.addr().get(), T::ALIGN);
                NonNull::slice_from_raw_parts(dst, len)
            }
        }
    }
}
unsafe impl Sealed for Unmanaged<'_> {
    const CONFIG: BumpConfig = DEFAULT_BUMP_CONFIG;

    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        Checkpoint::INVALID
    }

    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        _ = checkpoint;
    }

    #[inline(always)]
    fn pos(&self) -> *mut u8 {
        ptr::dangling_mut()
    }

    #[inline(always)]
    unsafe fn set_pos(&self, addr: usize) {
        _ = addr;
    }

    #[inline(always)]
    fn spec_alloc<E: ErrorBehavior>(&self, layout: Layout) -> Result<NonNull<u8>, E> {
        _ = layout;
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_sized<E: ErrorBehavior, T>(&self) -> Result<NonNull<T>, E> {
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_slice<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<T>, E> {
        _ = len;
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_slice_for<E: ErrorBehavior, T>(&self, slice: &[T]) -> Result<NonNull<T>, E> {
        _ = slice;
        Err(E::unmanaged())
    }

    #[inline(always)]
    fn spec_alloc_slice_greedy<E: ErrorBehavior, T>(&self, len: usize) -> Result<NonNull<[T]>, E> {
        _ = len;
        Err(E::unmanaged())
    }

    #[inline(always)]
    unsafe fn spec_shrink_slice<T>(&self, ptr: NonNull<T>, old_len: usize, new_len: usize) -> Option<NonNull<T>> {
        _ = (ptr, old_len, new_len);
        None
    }

    #[inline(always)]
    fn spec_prepare_sized_allocation<B: ErrorBehavior, T>(&self) -> Result<NonNull<T>, B> {
        Err(B::unmanaged())
    }

    #[inline(always)]
    fn spec_prepare_slice_allocation<E: ErrorBehavior, T>(&self, cap: usize) -> Result<NonNull<[T]>, E> {
        _ = cap;
        Err(E::unmanaged())
    }

    #[inline(always)]
    unsafe fn spec_alloc_prepared_slice<T>(&self, start: NonNull<T>, len: usize, cap: usize) -> NonNull<[T]> {
        _ = (start, len, cap);
        NonNull::slice_from_raw_parts(NonNull::dangling(), 0)
    }
}

/// TODO
///
/// # Safety
/// TODO
#[expect(private_bounds)]
pub unsafe trait BumpAllocator: Allocator + Sealed {
    /// Allocate an object.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc(123).into_boxu();
    /// assert_eq!(*allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc<T>(&self, value: T) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc(value));
        BumpAllocation(boxed)
    }

    /// Allocate an object.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc(123)?.into_boxu();
    /// assert_eq!(*allocated, 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc<T>(&self, value: T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc(value)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_with(|| 123).into_boxu();
    /// assert_eq!(*allocated, 123);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc_with(f));
        BumpAllocation(boxed)
    }

    /// Allocates space for an object, then calls `f` to produce the
    /// value to be put in that place.
    ///
    /// In some cases this could be more performant than `try_alloc(f())` because it
    /// permits the compiler to directly place `T` in the allocated memory instead of
    /// constructing it on the stack and copying it over.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_with(|| 123)?.into_boxu();
    /// assert_eq!(*allocated, 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc_with(f)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[alloc_with](BumpAllocator::alloc_with)(T::default)</code>.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_default::<i32>().into_boxu();
    /// assert_eq!(*allocated, 0);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_default<T: Default>(&self) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc_default());
        BumpAllocation(boxed)
    }

    /// Allocate an object with its default value.
    ///
    /// This is equivalent to <code>[try_alloc_with](BumpAllocator::try_alloc_with)(T::default)</code>.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_default::<i32>()?.into_boxu();
    /// assert_eq!(*allocated, 0);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_default<T: Default>(&self) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc_default()?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.alloc_clone(&[1, 2, 3]).into_boxu();
    /// assert_eq!(&*cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.alloc_clone("foo").into_boxu();
    /// assert_eq!(&*cloned, "foo");
    ///
    /// let cloned = bump.alloc_clone(c"foo").into_boxu();
    /// assert_eq!(&*cloned, c"foo");
    ///
    /// let cloned = bump.alloc_clone(Path::new("foo")).into_boxu();
    /// assert_eq!(&*cloned, Path::new("foo"));
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.alloc_clone(object).into_boxu();
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// ```
    #[inline(never)]
    #[cfg(feature = "panic-on-alloc")]
    #[cfg(feature = "nightly")]
    fn alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> BumpAllocation<T, &Self> {
        let boxed = panic_on_error(self.imp_alloc_clone(value));
        BumpAllocation(boxed)
    }

    /// Allocate an object by cloning it.
    ///
    /// Unlike `alloc(value.clone())` this method also works for dynamically-sized types.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    ///
    /// Allocate a `slice`, `str`, `CStr`, `Path`:
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use std::path::Path;
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let cloned = bump.try_alloc_clone(&[1, 2, 3])?.into_boxu();
    /// assert_eq!(&*cloned, &[1, 2, 3]);
    ///
    /// let cloned = bump.try_alloc_clone("foo")?.into_boxu();
    /// assert_eq!(&*cloned, "foo");
    ///
    /// let cloned = bump.try_alloc_clone(c"foo")?.into_boxu();
    /// assert_eq!(&*cloned, c"foo");
    ///
    /// let cloned = bump.try_alloc_clone(Path::new("foo"))?.into_boxu();
    /// assert_eq!(&*cloned, Path::new("foo"));
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Allocate a trait object:
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(clone_to_uninit)]
    ///
    /// use core::clone::CloneToUninit;
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// trait FnClone: Fn() -> String + CloneToUninit {}
    /// impl<T: ?Sized + Fn() -> String + CloneToUninit> FnClone for T {}
    ///
    /// // the closure references a local variable
    /// let reference = &String::from("Hello,");
    ///
    /// // and owns a string that it will have to clone
    /// let value = String::from("world!");
    ///
    /// let closure = move || format!("{reference} {value}");
    /// let object: &dyn FnClone = &closure;
    ///
    /// assert_eq!(object(), "Hello, world!");
    ///
    /// let bump: Bump = Bump::new();
    /// let object_clone = bump.try_alloc_clone(object)?.into_boxu();
    ///
    /// assert_eq!(object_clone(), "Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(never)]
    #[cfg(feature = "nightly")]
    fn try_alloc_clone<T: CloneToUninit + ?Sized>(&self, value: &T) -> Result<BumpAllocation<T, &Self>, AllocError> {
        let boxed = self.imp_alloc_clone(value)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`box_init`],
    /// `box_zero_init` <sup>[\[1\]](crate::zerocopy_08::box_zero_init) [\[2\]](crate::bytemuck::box_zero_init)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.alloc_uninit().into_boxu();
    /// let five = extra::box_init(uninit, 5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.alloc_uninit().into_boxu();
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit<T>(&self) -> BumpAllocation<MaybeUninit<T>, &Self> {
        let boxed = panic_on_error(self.imp_alloc_uninit());
        BumpAllocation(boxed)
    }

    /// Allocate an uninitialized object.
    ///
    /// You can safely initialize the object with [`box_init`],
    /// `box_zero_init` <sup>[\[1\]](crate::bytemuck::box_zero_init) [\[2\]](crate::zerocopy_08::box_zero_init)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.try_alloc_uninit()?.into_boxu();
    /// let five = extra::box_init(uninit, 5);
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.try_alloc_uninit()?.into_boxu();
    ///
    /// let five = unsafe {
    ///     uninit.write(5);
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit<T>(&self) -> Result<BumpAllocation<MaybeUninit<T>, &Self>, AllocError> {
        let boxed = self.imp_alloc_uninit()?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// // by value
    /// let a = bump.alloc_slice_move([1, 2]).into_boxu();
    /// let b = bump.alloc_slice_move(vec![3, 4]).into_boxu();
    /// let c = bump.alloc_slice_move(bump.alloc_iter(5..=6).into_boxu()).into_boxu();
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.alloc_slice_move(&mut other).into_boxu();
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(&*a, [1, 2]);
    /// assert_eq!(&*b, [3, 4]);
    /// assert_eq!(&*c, [5, 6]);
    /// assert_eq!(&*d, [7, 8]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_move(slice));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it by moving elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// // by value
    /// let a = bump.try_alloc_slice_move([1, 2])?.into_boxu();
    /// let b = bump.try_alloc_slice_move(vec![3, 4])?.into_boxu();
    /// let c = bump.try_alloc_slice_move(bump.alloc_iter(5..=6).into_boxu())?.into_boxu();
    ///
    /// // by mutable reference
    /// let mut other = vec![7, 8];
    /// let d = bump.try_alloc_slice_move(&mut other)?.into_boxu();
    /// assert!(other.is_empty());
    ///
    /// assert_eq!(&*a, [1, 2]);
    /// assert_eq!(&*b, [3, 4]);
    /// assert_eq!(&*c, [5, 6]);
    /// assert_eq!(&*d, [7, 8]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_move<T>(&self, slice: impl OwnedSlice<Item = T>) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_move(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]).into_boxu();
    /// assert_eq!(&*allocated, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_copy(slice));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it by `Copy`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_copy(&[1, 2, 3])?.into_boxu();
    /// assert_eq!(&*allocated, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_copy<T: Copy>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_copy(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]).into_boxu();
    /// assert_eq!(&*allocated, [String::from("a"), String::from("b")]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_clone(slice));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it by `Clone`ing elements from an existing slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_clone(&[String::from("a"), String::from("b")])?.into_boxu();
    /// assert_eq!(&*allocated, [String::from("a"), String::from("b")]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_clone<T: Clone>(&self, slice: &[T]) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_clone(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_fill(3, "ho").into_boxu();
    /// assert_eq!(&*allocated, ["ho", "ho", "ho"]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_fill(len, value));
        BumpAllocation(boxed)
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_fill(3, "ho")?.into_boxu();
    /// assert_eq!(&*allocated, ["ho", "ho", "ho"]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_fill<T: Clone>(&self, len: usize, value: T) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_fill(len, value)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](BumpAllocator::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default).into_boxu();
    /// assert_eq!(&*allocated, [0, 0, 0]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_slice_fill_with<T>(&self, len: usize, f: impl FnMut() -> T) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_slice_fill_with(len, f));
        BumpAllocation(boxed)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`try_alloc_slice_fill`](BumpAllocator::try_alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_slice_fill_with::<i32>(3, Default::default)?.into_boxu();
    /// assert_eq!(&*allocated, [0, 0, 0]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_slice_fill_with<T>(
        &self,
        len: usize,
        f: impl FnMut() -> T,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_slice_fill_with(len, f)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`box_init_fill`],
    /// [`box_init_fill_with`],
    /// [`box_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra::box_init_copy};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.alloc_uninit_slice(3).into_boxu();
    /// let values = box_init_copy(uninit, &[1, 2, 3]);
    ///
    /// assert_eq!(&*values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.alloc_uninit_slice(3).into_boxu();
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit_slice<T>(&self, len: usize) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxed = panic_on_error(self.imp_alloc_uninit_slice(len));
        BumpAllocation(boxed)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`box_init_fill`],
    /// [`box_init_fill_with`],
    /// [`box_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// Safely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator, extra::box_init_copy};
    /// let bump: Bump = Bump::new();
    ///
    /// let uninit = bump.try_alloc_uninit_slice(3)?.into_boxu();
    /// let values = box_init_copy(uninit, &[1, 2, 3]);
    ///
    /// assert_eq!(&*values, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let mut uninit = bump.try_alloc_uninit_slice(3)?.into_boxu();
    ///
    /// let values = unsafe {
    ///     uninit[0].write(1);
    ///     uninit[1].write(2);
    ///     uninit[2].write(3);
    ///
    ///     uninit.assume_init()
    /// };
    ///
    /// assert_eq!(*values, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit_slice<T>(&self, len: usize) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxed = self.imp_alloc_uninit_slice(len)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`box_init_fill`],
    /// [`box_init_fill_with`],
    /// [`box_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// This is just like [`alloc_uninit_slice`](BumpAllocator::alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.alloc_uninit_slice_for(slice).into_boxu();
    /// assert_eq!(uninit_slice.len(), 3);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_uninit_slice_for<T>(&self, slice: &[T]) -> BumpAllocation<[MaybeUninit<T>], &Self> {
        let boxed = panic_on_error(self.imp_alloc_uninit_slice_for(slice));
        BumpAllocation(boxed)
    }

    /// Allocate an uninitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`box_init_fill`],
    /// [`box_init_fill_with`],
    /// [`box_init_clone`],
    /// `box_zero_init_slice` <sup>[\[1\]](crate::zerocopy_08::box_zero_init_slice) [\[2\]](crate::bytemuck::box_zero_init_slice)</sup>
    /// or unsafely with [`assume_init`](Box::assume_init).
    ///
    /// This is just like [`try_alloc_uninit_slice`](BumpAllocator::try_alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = &[1, 2, 3];
    /// let uninit_slice = bump.try_alloc_uninit_slice_for(slice)?.into_boxu();
    /// assert_eq!(uninit_slice.len(), 3);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_uninit_slice_for<T>(&self, slice: &[T]) -> Result<BumpAllocation<[MaybeUninit<T>], &Self>, AllocError> {
        let boxed = self.imp_alloc_uninit_slice_for(slice)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `str`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_str("Hello, world!").into_ref();
    /// assert_eq!(allocated, "Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_str(&self, src: &str) -> BumpAllocation<str, &Self> {
        let boxed = panic_on_error(self.imp_alloc_str(src));
        BumpAllocation(boxed)
    }

    /// Allocate a `str`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_str("Hello, world!")?.into_ref();
    /// assert_eq!(allocated, "Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_str(&self, src: &str) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxed = self.imp_alloc_str(src)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`alloc_fmt_mut`](MutBumpAllocator::alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two)).into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_fmt(&self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxed = panic_on_error(self.imp_alloc_fmt(args));
        BumpAllocation(boxed)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_fmt_mut`](MutBumpAllocator::try_alloc_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_fmt(&self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxed = self.imp_alloc_fmt(args)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `CStr`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_cstr(c"Hello, world!").into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr(&self, src: &CStr) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr(src))
    }

    /// Allocate a `CStr`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_cstr(c"Hello, world!")?.into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr(&self, src: &CStr) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr(src)
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.alloc_cstr_from_str("Hello, world!").into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def").into_ref();
    /// assert_eq!(allocated, c"abc");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_from_str(&self, src: &str) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr_from_str(src))
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("Hello, world!")?.into_ref();
    /// assert_eq!(allocated, c"Hello, world!");
    ///
    /// let allocated = bump.try_alloc_cstr_from_str("abc\0def")?.into_ref();
    /// assert_eq!(allocated, c"abc");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr_from_str(&self, src: &str) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr_from_str(src)
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`alloc_cstr_fmt_mut`](MutBumpAllocator::alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two)).into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}")).into_ref();
    /// assert_eq!(one, c"1");
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_fmt(&self, args: fmt::Arguments) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr_fmt(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// If you have a `&mut self` you can use [`try_alloc_cstr_fmt_mut`](MutBumpAllocator::try_alloc_cstr_fmt_mut)
    /// instead for better performance.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt(format_args!("{one}\0{two}"))?.into_ref();
    /// assert_eq!(one, c"1");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_cstr_fmt(&self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr_fmt(args)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have a `&mut self` you can get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.alloc_iter([1, 2, 3]).into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter_mut`]: MutBumpAllocator::alloc_iter_mut
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_iter(iter));
        BumpAllocation(boxed)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have a `&mut self` you can get somewhat better performance by using [`try_alloc_iter_mut`].
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.try_alloc_iter([1, 2, 3])?.into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter_mut`]: MutBumpAllocator::try_alloc_iter_mut
    #[inline(always)]
    fn try_alloc_iter<T>(&self, iter: impl IntoIterator<Item = T>) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_iter(iter)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.alloc_iter_exact([1, 2, 3]).into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// ```
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter_exact<T, I>(&self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpAllocation<[T], &Self>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let boxed = panic_on_error(self.imp_alloc_iter_exact(iter));
        BumpAllocation(boxed)
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let slice = bump.try_alloc_iter_exact([1, 2, 3])?.into_boxu();
    /// assert_eq!(&*slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    #[inline(always)]
    fn try_alloc_iter_exact<T, I>(
        &self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError>
    where
        I: ExactSizeIterator<Item = T>,
    {
        let boxed = self.imp_alloc_iter_exact(iter)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`alloc_try_with_mut`], optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(bump.stats().allocated(), size_of::<Result<i32, i32>>());
    /// assert_eq!(*result.unwrap().into_boxu(), 123);
    /// ```
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.alloc_try_with(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(bump.stats().allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// ```
    ///
    /// [`alloc_try_with_mut`]: MutBumpAllocator::alloc_try_with_mut
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        match panic_on_error(self.imp_alloc_try_with(f)) {
            Ok(boxed) => Ok(BumpAllocation(boxed)),
            Err(err) => Err(err),
        }
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// There is also [`try_alloc_try_with_mut`], optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(bump.stats().allocated(), size_of::<Result<i32, i32>>());
    /// assert_eq!(*result.unwrap().into_boxu(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let result = bump.try_alloc_try_with(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(bump.stats().allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_try_with_mut`]: MutBumpAllocator::try_alloc_try_with_mut
    #[inline(always)]
    fn try_alloc_try_with<T, E>(
        &self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.imp_alloc_try_with(f)? {
            Ok(boxed) => Ok(BumpAllocation(boxed)),
            Err(err) => Err(err),
        })
    }

    /// TODO
    ///
    /// # Panics
    /// TODO
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_vec_greedy<T>(&self, len: usize) -> Vec<T, &Self> {
        panic_on_error(self.imp_alloc_vec_greedy(len))
    }

    /// TODO
    ///
    /// # Errors
    /// TODO
    fn try_alloc_vec_greedy<T>(&self, len: usize) -> Result<Vec<T, &Self>, AllocError> {
        self.imp_alloc_vec_greedy(len)
    }

    /// Creates a checkpoint of the current bump position.
    ///
    /// The bump position can be reset to this checkpoint with [`reset_to`].
    ///
    /// [`reset_to`]: BumpAllocator::reset_to
    #[must_use]
    #[inline(always)]
    fn checkpoint(&self) -> Checkpoint {
        Sealed::checkpoint(self)
    }

    /// Resets the bump position to a previously created checkpoint.
    ///
    /// The memory that has been allocated since then will be reused by future allocations.
    ///
    /// # Safety
    ///
    /// - This method invalidates all memory blocks allocated since creation of this checkpoint.
    /// - The provided checkpoint must have been created by this bump allocator.
    /// - The bump allocator must not have been [`reset`] since creation of this checkpoint.
    /// - The checkpoint must not have been created by a bump allocator with no chunks when self is **not** [`const_constructible`].
    /// - The checkpoint must not have been created by a [claimed] bump allocator when self is **not** [`claimable`].
    ///
    /// [`reset`]: crate::Bump::reset
    /// [claimed]: crate::BumpScope::claim
    /// [`claimable`]: crate::config::BumpConfigBuilder::claimable
    /// [`const_constructible`]: crate::config::BumpConfigBuilder::const_constructible
    ///
    /// # Examples
    ///
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    ///
    /// let checkpoint = bump.checkpoint();
    ///
    /// {
    ///     let hello = bump.alloc_str("hello");
    ///     assert_eq!(bump.stats().allocated(), 5);
    ///     # _ = hello;
    /// }
    ///
    /// unsafe { bump.reset_to(checkpoint); }
    /// assert_eq!(bump.stats().allocated(), 0);
    /// ```
    #[inline(always)]
    unsafe fn reset_to(&self, checkpoint: Checkpoint) {
        unsafe { Sealed::reset_to(self, checkpoint) }
    }

    /// Returns true if the bump allocator is currently [claimed].
    ///
    /// [claimed]: crate::BumpScope::claim
    #[must_use]
    fn is_claimed(&self) -> bool;

    /// TODO
    fn any_stats(&self) -> AnyStats<'_>;

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these bytes are not necessarily in one contiguous region but
    /// might be spread across multiple chunks.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// Panics if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    ///
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    ///
    /// [stats]: crate::BumpScope::stats
    /// [remaining]: crate::stats::Stats::remaining
    /// [claimed]: crate::BumpScope::claim
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize);

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve`, <code>self.[stats][]().[remaining][]()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that these bytes are not necessarily in one contiguous region but
    /// might be spread across multiple chunks.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// Errors if the bump allocator is currently [claimed].
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, BumpAllocator};
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.try_reserve(4096)?;
    /// assert!(bump.stats().capacity() >= 4096);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [stats]: crate::BumpScope::stats
    /// [remaining]: crate::stats::Stats::remaining
    /// [claimed]: crate::BumpScope::claim
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError>;
}
unsafe impl<T: BumpAllocator> BumpAllocator for &T {
    #[inline(always)]
    fn is_claimed(&self) -> bool {
        T::is_claimed(self)
    }

    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        T::any_stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        T::reserve(self, additional);
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        T::try_reserve(self, additional)
    }
}
unsafe impl<T: BumpAllocator> BumpAllocator for &mut T {
    #[inline(always)]
    fn is_claimed(&self) -> bool {
        T::is_claimed(self)
    }

    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        T::any_stats(self)
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        T::reserve(self, additional);
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        T::try_reserve(self, additional)
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> BumpAllocator for Bump<A, C> {
    #[inline(always)]
    fn is_claimed(&self) -> bool {
        self.raw.is_claimed()
    }

    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        self.stats().into()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        panic_on_error(self.raw.reserve(additional));
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        self.raw.reserve(additional)
    }
}
unsafe impl<A: Allocator, const C: BumpConfig> BumpAllocator for BumpScope<'_, A, C> {
    #[inline(always)]
    fn is_claimed(&self) -> bool {
        self.raw.is_claimed()
    }

    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        self.stats().into()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        panic_on_error(self.raw.reserve(additional));
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        self.raw.reserve(additional)
    }
}
unsafe impl BumpAllocator for Unmanaged<'_> {
    #[inline(always)]
    fn is_claimed(&self) -> bool {
        false
    }

    #[inline(always)]
    fn any_stats(&self) -> AnyStats<'_> {
        AnyStats::default()
    }

    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn reserve(&self, additional: usize) {
        _ = additional;
        error_behavior::panic::unmanaged();
    }

    #[inline(always)]
    fn try_reserve(&self, additional: usize) -> Result<(), AllocError> {
        _ = additional;
        Err(AllocError)
    }
}

/// TODO
///
/// # Safety
/// TODO
#[expect(private_bounds)]
pub unsafe trait BumpAllocatorScope<'a>: BumpAllocator + Sealed {}
unsafe impl<'a, T: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &T {}
unsafe impl<'a, T: BumpAllocatorScope<'a>> BumpAllocatorScope<'a> for &mut T {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for &'a Bump<A, C> {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for &'a mut Bump<A, C> {}
unsafe impl<'a, A: Allocator, const C: BumpConfig> BumpAllocatorScope<'a> for BumpScope<'a, A, C> {}
unsafe impl<'a> BumpAllocatorScope<'a> for Unmanaged<'a> {}

/// TODO
///
/// # Safety
/// TODO
#[expect(private_bounds)]
pub unsafe trait MutBumpAllocator: BumpAllocator + Sealed {
    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let slice = bump.alloc_iter_mut([1, 2, 3]).into_ref();
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    ///
    /// [`alloc_iter`]: BumpAllocator::alloc_iter
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_iter_mut<T>(&mut self, iter: impl IntoIterator<Item = T>) -> BumpAllocation<[T], &Self> {
        let boxed = panic_on_error(self.imp_alloc_iter_mut(iter));
        BumpAllocation(boxed)
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_iter`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary vector used for the allocation. As a result, that vector rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let slice = bump.try_alloc_iter_mut([1, 2, 3])?.into_ref();
    /// assert_eq!(slice, [1, 2, 3]);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_iter`]: BumpAllocator::alloc_iter
    #[inline(always)]
    fn try_alloc_iter_mut<T>(
        &mut self,
        iter: impl IntoIterator<Item = T>,
    ) -> Result<BumpAllocation<[T], &Self>, AllocError> {
        let boxed = self.imp_alloc_iter_mut(iter)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two)).into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    ///
    /// [`alloc_fmt`]: BumpAllocator::alloc_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_fmt_mut(&mut self, args: fmt::Arguments) -> BumpAllocation<str, &Self> {
        let boxed = panic_on_error(self.imp_alloc_fmt_mut(args));
        BumpAllocation(boxed)
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_fmt`]: BumpAllocator::try_alloc_fmt
    #[inline(always)]
    fn try_alloc_fmt_mut(&mut self, args: fmt::Arguments) -> Result<BumpAllocation<str, &Self>, AllocError> {
        let boxed = self.imp_alloc_fmt_mut(args)?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`alloc_cstr_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// This technically also panics if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two)).into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt_mut(format_args!("{one}\0{two}")).into_ref();
    /// assert_eq!(one, c"1");
    /// ```
    ///
    /// [`alloc_cstr_fmt`]: BumpAllocator::alloc_cstr_fmt
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    fn alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> IntoRef<CStr, &Self> {
        panic_on_error(self.imp_alloc_cstr_fmt_mut(args))
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop at the first `'\0'`.
    ///
    /// This function is designed as a performance improvement over [`try_alloc_cstr_fmt`].
    /// By taking `self` as `&mut`, it can use the entire remaining chunk space as the capacity
    /// for the temporary string buffer used for the allocation. As a result, that string buffer rarely needs to grow.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// This technically also errors if the `fmt()` implementation returned an Error,
    /// but since [`fmt()` implementors should only error when writing to the stream fails](core::fmt::Error),
    /// that should be equivalent to an allocation failure.
    ///
    /// # Examples
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    ///
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.try_alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two))?.into_ref();
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.try_alloc_cstr_fmt_mut(format_args!("{one}\0{two}"))?.into_ref();
    /// assert_eq!(one, c"1");
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_cstr_fmt`]: BumpAllocator::try_alloc_cstr_fmt
    #[inline(always)]
    fn try_alloc_cstr_fmt_mut(&mut self, args: fmt::Arguments) -> Result<IntoRef<CStr, &Self>, AllocError> {
        self.imp_alloc_cstr_fmt_mut(args)
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`alloc_try_with`], but optimized for a mutable reference.
    ///
    /// # Panics
    /// Panics if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(offset_of_enum)]
    /// use core::mem::offset_of;
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) });
    /// assert_eq!(stats.allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// assert_eq!(*result.unwrap().into_box(), 123);
    /// ```
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) });
    /// assert_eq!(stats.allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// ```
    ///
    /// [`alloc_try_with`]: BumpAllocator::alloc_try_with
    #[inline(always)]
    #[cfg(feature = "panic-on-alloc")]
    #[expect(clippy::missing_errors_doc)]
    fn alloc_try_with_mut<T, E>(&mut self, f: impl FnOnce() -> Result<T, E>) -> Result<BumpAllocation<T, &Self>, E> {
        let boxed = panic_on_error(self.imp_alloc_try_with_mut(f))?;
        Ok(BumpAllocation(boxed))
    }

    /// Allocates the result of `f` in the bump allocator, then moves `E` out of it and deallocates the space it took up.
    ///
    /// This can be more performant than allocating `T` after the fact, as `Result<T, E>` may be constructed in the bump allocators memory instead of on the stack and then copied over.
    ///
    /// This is just like [`try_alloc_try_with`], but optimized for a mutable reference.
    ///
    /// # Errors
    /// Errors if the allocation fails.
    ///
    /// # Examples
    #[cfg_attr(feature = "nightly", doc = "```")]
    #[cfg_attr(not(feature = "nightly"), doc = "```ignore")]
    /// #![feature(offset_of_enum)]
    /// use core::mem::offset_of;
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Ok(123) })?;
    /// assert_eq!(stats.allocated(), offset_of!(Result<i32, i32>, Ok.0) + size_of::<i32>());
    /// assert_eq!(*result.unwrap().into_box(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    /// ```
    /// use bump_scope::{Bump, MutBumpAllocator};
    /// let mut bump: Bump = Bump::new();
    /// let bump = bump.as_mut_scope();
    /// let stats = bump.stats();
    ///
    /// let result = bump.try_alloc_try_with_mut(|| -> Result<i32, i32> { Err(123) })?;
    /// assert_eq!(stats.allocated(), 0);
    /// assert_eq!(result.unwrap_err(), 123);
    /// # Ok::<(), ::core::alloc::AllocError>(())
    /// ```
    ///
    /// [`try_alloc_try_with`]: BumpAllocator::try_alloc_try_with
    #[inline(always)]
    fn try_alloc_try_with_mut<T, E>(
        &mut self,
        f: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<BumpAllocation<T, &Self>, E>, AllocError> {
        Ok(match self.imp_alloc_try_with_mut(f)? {
            Ok(boxed) => Ok(BumpAllocation(boxed)),
            Err(err) => Err(err),
        })
    }
}
unsafe impl<T: MutBumpAllocator> MutBumpAllocator for &mut T {}
unsafe impl<A: Allocator, const C: BumpConfig> MutBumpAllocator for Bump<A, C> {}
unsafe impl<A: Allocator, const C: BumpConfig> MutBumpAllocator for BumpScope<'_, A, C> {}

/// TODO
///
/// # Safety
/// TODO
#[expect(private_bounds)]
pub unsafe trait MutBumpAllocatorScope<'a>: MutBumpAllocator + BumpAllocatorScope<'a> + Sealed {}
unsafe impl<'a, T: MutBumpAllocator + BumpAllocatorScope<'a>> MutBumpAllocatorScope<'a> for T {}

/// TODO
pub trait AsBumpScope<'a, A, const C: BumpConfig> {
    /// TODO
    fn as_bump_scope(&self) -> &BumpScope<'a, A, C>;
}

impl<'a, A: Allocator, const C: BumpConfig> AsBumpScope<'a, A, C> for &'a Bump<A, C> {
    fn as_bump_scope(&self) -> &BumpScope<'a, A, C> {
        unsafe { transmute_ref::<Bump<A, C>, BumpScope<'a, A, C>>(*self) }
    }
}

impl<'a, A: Allocator, const C: BumpConfig> AsBumpScope<'a, A, C> for BumpScope<'a, A, C> {
    fn as_bump_scope(&self) -> &BumpScope<'a, A, C> {
        self
    }
}

/// TODO
pub trait AsMutBumpScope<'a, A, const C: BumpConfig> {
    /// TODO
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'a, A, C>;
}

impl<'a, A: Allocator, const C: BumpConfig> AsMutBumpScope<'a, A, C> for &'a mut Bump<A, C> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'a, A, C> {
        unsafe { transmute_mut::<Bump<A, C>, BumpScope<'a, A, C>>(*self) }
    }
}

impl<'a, A: Allocator, const C: BumpConfig> AsMutBumpScope<'a, A, C> for BumpScope<'a, A, C> {
    fn as_mut_bump_scope(&mut self) -> &mut BumpScope<'a, A, C> {
        self
    }
}

macro_rules! assert_implements {
    ([$($what:tt)*] $($ty:ty)*) => {
        #[cfg(test)]
        const _: () = {
            #[allow(dead_code)]
            type A = crate::NoopAllocator;
            #[allow(dead_code)]
            type Bump = crate::Bump<A>;
            #[allow(dead_code)]
            type BumpScope<'a> = crate::BumpScope<'a, A>;
            #[allow(clippy::extra_unused_lifetimes)]
            const fn implements<'a, What: $($what)*>() {}
            $(
                #[allow(clippy::mut_mut)]
                implements::<$ty>();
            )*
        };
    };
}

assert_implements! {
    [BumpAllocator]

    Bump
    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [BumpAllocatorScope<'a>]

    &Bump
    &&Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &BumpScope
    &&BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [MutBumpAllocator]

    Bump
    &mut Bump
    &mut &mut Bump

    BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}

assert_implements! {
    [MutBumpAllocatorScope<'a>]

    &mut Bump
    &mut &mut Bump

    BumpScope
    &mut BumpScope
    &mut &mut BumpScope
}
