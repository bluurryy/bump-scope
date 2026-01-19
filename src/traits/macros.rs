/// Add trait methods as methods to the struct, so users don't have to import
/// the traits to access the methods and don't have to write `bump.as_scope().alloc(...)`
/// or `(&bump).alloc(...)` to allocate on a `Bump`.
///
/// Would be cool if there was a way to mark the trait impls in a way to make
/// all the methods available for the struct without importing the trait,
/// like <https://internals.rust-lang.org/t/fundamental-impl-trait-for-type/19201>.
macro_rules! forward_methods {
    (
        self: $self:ident
        access: {$access:expr}
        access_mut: {$access_mut:expr}
        lifetime: $lifetime:lifetime
    ) => {
        /// Forwards to [`BumpAllocatorCore::checkpoint`].
        #[inline(always)]
        pub fn checkpoint(&$self) -> Checkpoint {
            BumpAllocatorCore::checkpoint($access)
        }

        /// Forwards to [`BumpAllocatorCore::reset_to`].
        #[inline(always)]
        pub unsafe fn reset_to(&$self, checkpoint: Checkpoint) {
            unsafe { BumpAllocatorCore::reset_to($access, checkpoint) }
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc<T>(&$self, value: T) -> BumpBox<$lifetime, T> {
            BumpAllocatorTypedScope::alloc($access, value)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc`].
        #[inline(always)]
        pub fn try_alloc<T>(&$self, value: T) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorTypedScope::try_alloc($access, value)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_with`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_with<T>(&$self, f: impl FnOnce() -> T) -> BumpBox<$lifetime, T> {
            BumpAllocatorTypedScope::alloc_with($access, f)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_with`].
        #[inline(always)]
        pub fn try_alloc_with<T>(&$self, f: impl FnOnce() -> T) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_with($access, f)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_default`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_default<T: Default>(&$self) -> BumpBox<$lifetime, T> {
            BumpAllocatorTypedScope::alloc_default($access)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_default`].
        #[inline(always)]
        pub fn try_alloc_default<T: Default>(&$self) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_default($access)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_clone`].
        #[inline(always)]
        #[cfg(feature = "nightly-clone-to-uninit")]
        pub fn alloc_clone<T: CloneToUninit + ?Sized>(&$self, value: &T) -> BumpBox<$lifetime, T> {
            BumpAllocatorTypedScope::alloc_clone($access, value)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_clone`].
        #[inline(always)]
        #[cfg(feature = "nightly-clone-to-uninit")]
        pub fn try_alloc_clone<T: CloneToUninit + ?Sized>(&$self, value: &T) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_clone($access, value)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_slice_move`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_move<T>(&$self, slice: impl OwnedSlice<Item = T>) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorTypedScope::alloc_slice_move($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_slice_move`].
        #[inline(always)]
        pub fn try_alloc_slice_move<T>(&$self, slice: impl OwnedSlice<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_slice_move($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_slice_copy`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_copy<T: Copy>(&$self, slice: &[T]) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorTypedScope::alloc_slice_copy($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_slice_copy`].
        #[inline(always)]
        pub fn try_alloc_slice_copy<T: Copy>(&$self, slice: &[T]) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_slice_copy($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_slice_clone`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_clone<T: Clone>(&$self, slice: &[T]) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorTypedScope::alloc_slice_clone($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_slice_clone`].
        #[inline(always)]
        pub fn try_alloc_slice_clone<T: Clone>(&$self, slice: &[T]) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_slice_clone($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_slice_fill`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_fill<T: Clone>(&$self, len: usize, value: T) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorTypedScope::alloc_slice_fill($access, len, value)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_slice_fill`].
        #[inline(always)]
        pub fn try_alloc_slice_fill<T: Clone>(&$self, len: usize, value: T) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_slice_fill($access, len, value)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_slice_fill_with`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_fill_with<T>(&$self, len: usize, f: impl FnMut() -> T) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorTypedScope::alloc_slice_fill_with($access, len, f)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_slice_fill_with`].
        #[inline(always)]
        pub fn try_alloc_slice_fill_with<T>(&$self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_slice_fill_with($access, len, f)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_str`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_str(&$self, src: &str) -> BumpBox<$lifetime, str> {
            BumpAllocatorTypedScope::alloc_str($access, src)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_str`].
        #[inline(always)]
        pub fn try_alloc_str(&$self, src: &str) -> Result<BumpBox<$lifetime, str>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_str($access, src)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_fmt`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_fmt(&$self, args: fmt::Arguments) -> BumpBox<$lifetime, str> {
            BumpAllocatorTypedScope::alloc_fmt($access, args)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_fmt`].
        #[inline(always)]
        pub fn try_alloc_fmt(&$self, args: fmt::Arguments) -> Result<BumpBox<$lifetime, str>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_fmt($access, args)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::alloc_fmt_mut`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_fmt_mut(&mut $self, args: fmt::Arguments) -> BumpBox<$lifetime, str> {
            MutBumpAllocatorTypedScope::alloc_fmt_mut($access_mut, args)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::try_alloc_fmt_mut`].
        #[inline(always)]
        pub fn try_alloc_fmt_mut(&mut $self, args: fmt::Arguments) -> Result<BumpBox<$lifetime, str>, AllocError> {
            MutBumpAllocatorTypedScope::try_alloc_fmt_mut($access_mut, args)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_cstr`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr(&$self, src: &CStr) -> &$lifetime CStr {
            BumpAllocatorTypedScope::alloc_cstr($access, src)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_cstr`].
        #[inline(always)]
        pub fn try_alloc_cstr(&$self, src: &CStr) -> Result<&$lifetime CStr, AllocError> {
            BumpAllocatorTypedScope::try_alloc_cstr($access, src)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_cstr_from_str`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr_from_str(&$self, src: &str) -> &$lifetime CStr {
            BumpAllocatorTypedScope::alloc_cstr_from_str($access, src)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_cstr_from_str`].
        #[inline(always)]
        pub fn try_alloc_cstr_from_str(&$self, src: &str) -> Result<&$lifetime CStr, AllocError> {
            BumpAllocatorTypedScope::try_alloc_cstr_from_str($access, src)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_cstr_fmt`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr_fmt(&$self, args: fmt::Arguments) -> &$lifetime CStr {
            BumpAllocatorTypedScope::alloc_cstr_fmt($access, args)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_cstr_fmt`].
        #[inline(always)]
        pub fn try_alloc_cstr_fmt(&$self, args: fmt::Arguments) -> Result<&$lifetime CStr, AllocError> {
            BumpAllocatorTypedScope::try_alloc_cstr_fmt($access, args)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::alloc_cstr_fmt_mut`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr_fmt_mut(&mut $self, args: fmt::Arguments) -> &$lifetime CStr {
            MutBumpAllocatorTypedScope::alloc_cstr_fmt_mut($access_mut, args)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::try_alloc_cstr_fmt_mut`].
        #[inline(always)]
        pub fn try_alloc_cstr_fmt_mut(&mut $self, args: fmt::Arguments) -> Result<&$lifetime CStr, AllocError> {
            MutBumpAllocatorTypedScope::try_alloc_cstr_fmt_mut($access_mut, args)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_iter`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter<T>(&$self, iter: impl IntoIterator<Item = T>) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorTypedScope::alloc_iter($access, iter)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_iter`].
        #[inline(always)]
        pub fn try_alloc_iter<T>(&$self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_iter($access, iter)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_iter_exact`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter_exact<T, I>(&$self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<$lifetime, [T]>
        where
            I: ExactSizeIterator<Item = T>,
        {
            BumpAllocatorTypedScope::alloc_iter_exact($access, iter)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_iter_exact`].
        #[inline(always)]
        pub fn try_alloc_iter_exact<T, I>(
            &$self,
            iter: impl IntoIterator<Item = T, IntoIter = I>,
        ) -> Result<BumpBox<$lifetime, [T]>, AllocError>
        where
            I: ExactSizeIterator<Item = T>,
        {
            BumpAllocatorTypedScope::try_alloc_iter_exact($access, iter)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::alloc_iter_mut`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter_mut<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> BumpBox<$lifetime, [T]> {
            MutBumpAllocatorTypedScope::alloc_iter_mut($access_mut, iter)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::try_alloc_iter_mut`].
        #[inline(always)]
        pub fn try_alloc_iter_mut<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            MutBumpAllocatorTypedScope::try_alloc_iter_mut($access_mut, iter)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::alloc_iter_mut_rev`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter_mut_rev<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> BumpBox<$lifetime, [T]> {
            MutBumpAllocatorTypedScope::alloc_iter_mut_rev($access_mut, iter)
        }

        /// Forwards to [`MutBumpAllocatorTypedScope::try_alloc_iter_mut_rev`].
        #[inline(always)]
        pub fn try_alloc_iter_mut_rev<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            MutBumpAllocatorTypedScope::try_alloc_iter_mut_rev($access_mut, iter)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_uninit`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_uninit<T>(&$self) -> BumpBox<$lifetime, MaybeUninit<T>> {
            BumpAllocatorTypedScope::alloc_uninit($access)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_uninit`].
        #[inline(always)]
        pub fn try_alloc_uninit<T>(&$self) -> Result<BumpBox<$lifetime, MaybeUninit<T>>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_uninit($access)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_uninit_slice`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_uninit_slice<T>(&$self, len: usize) -> BumpBox<$lifetime, [MaybeUninit<T>]> {
            BumpAllocatorTypedScope::alloc_uninit_slice($access, len)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_uninit_slice`].
        #[inline(always)]
        pub fn try_alloc_uninit_slice<T>(&$self, len: usize) -> Result<BumpBox<$lifetime, [MaybeUninit<T>]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_uninit_slice($access, len)
        }

        /// Forwards to [`BumpAllocatorTypedScope::alloc_uninit_slice_for`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_uninit_slice_for<T>(&$self, slice: &[T]) -> BumpBox<$lifetime, [MaybeUninit<T>]> {
            BumpAllocatorTypedScope::alloc_uninit_slice_for($access, slice)
        }

        /// Forwards to [`BumpAllocatorTypedScope::try_alloc_uninit_slice_for`].
        #[inline(always)]
        pub fn try_alloc_uninit_slice_for<T>(&$self, slice: &[T]) -> Result<BumpBox<$lifetime, [MaybeUninit<T>]>, AllocError> {
            BumpAllocatorTypedScope::try_alloc_uninit_slice_for($access, slice)
        }

        /// Forwards to [`BumpAllocatorTyped::dealloc`].
        #[inline(always)]
        pub fn dealloc<T: ?Sized>(&$self, boxed: BumpBox<T>) {
            BumpAllocatorTyped::dealloc($access, boxed);
        }

        /// Forwards to [`BumpAllocatorTyped::reserve_bytes`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn reserve_bytes(&$self, additional: usize) {
            BumpAllocatorTyped::reserve_bytes($access, additional);
        }

        /// Forwards to [`BumpAllocatorTyped::try_reserve_bytes`].
        #[inline(always)]
        pub fn try_reserve_bytes(&$self, additional: usize) -> Result<(), AllocError> {
            BumpAllocatorTyped::try_reserve_bytes($access, additional)
        }
    };
}

pub(crate) use forward_methods;
