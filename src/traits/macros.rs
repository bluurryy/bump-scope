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
        /// Forwards to [`BumpAllocatorCore::is_claimed`].
        #[must_use]
        #[inline(always)]
        pub fn is_claimed(&self) -> bool {
            BumpAllocatorCore::is_claimed(self)
        }

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

        /// Forwards to [`BumpAllocatorScope::alloc`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc<T>(&$self, value: T) -> BumpBox<$lifetime, T> {
            BumpAllocatorScope::alloc($access, value)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc`].
        #[inline(always)]
        pub fn try_alloc<T>(&$self, value: T) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorScope::try_alloc($access, value)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_with`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_with<T>(&$self, f: impl FnOnce() -> T) -> BumpBox<$lifetime, T> {
            BumpAllocatorScope::alloc_with($access, f)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_with`].
        #[inline(always)]
        pub fn try_alloc_with<T>(&$self, f: impl FnOnce() -> T) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorScope::try_alloc_with($access, f)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_default`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_default<T: Default>(&$self) -> BumpBox<$lifetime, T> {
            BumpAllocatorScope::alloc_default($access)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_default`].
        #[inline(always)]
        pub fn try_alloc_default<T: Default>(&$self) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorScope::try_alloc_default($access)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_clone`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        #[cfg(feature = "nightly-clone-to-uninit")]
        pub fn alloc_clone<T: CloneToUninit + ?Sized>(&$self, value: &T) -> BumpBox<$lifetime, T> {
            BumpAllocatorScope::alloc_clone($access, value)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_clone`].
        #[inline(always)]
        #[cfg(feature = "nightly-clone-to-uninit")]
        pub fn try_alloc_clone<T: CloneToUninit + ?Sized>(&$self, value: &T) -> Result<BumpBox<$lifetime, T>, AllocError> {
            BumpAllocatorScope::try_alloc_clone($access, value)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_slice_move`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_move<T>(&$self, slice: impl OwnedSlice<Item = T>) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorScope::alloc_slice_move($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_slice_move`].
        #[inline(always)]
        pub fn try_alloc_slice_move<T>(&$self, slice: impl OwnedSlice<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorScope::try_alloc_slice_move($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_slice_copy`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_copy<T: Copy>(&$self, slice: &[T]) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorScope::alloc_slice_copy($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_slice_copy`].
        #[inline(always)]
        pub fn try_alloc_slice_copy<T: Copy>(&$self, slice: &[T]) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorScope::try_alloc_slice_copy($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_slice_clone`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_clone<T: Clone>(&$self, slice: &[T]) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorScope::alloc_slice_clone($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_slice_clone`].
        #[inline(always)]
        pub fn try_alloc_slice_clone<T: Clone>(&$self, slice: &[T]) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorScope::try_alloc_slice_clone($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_slice_fill`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_fill<T: Clone>(&$self, len: usize, value: T) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorScope::alloc_slice_fill($access, len, value)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_slice_fill`].
        #[inline(always)]
        pub fn try_alloc_slice_fill<T: Clone>(&$self, len: usize, value: T) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorScope::try_alloc_slice_fill($access, len, value)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_slice_fill_with`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_slice_fill_with<T>(&$self, len: usize, f: impl FnMut() -> T) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorScope::alloc_slice_fill_with($access, len, f)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_slice_fill_with`].
        #[inline(always)]
        pub fn try_alloc_slice_fill_with<T>(&$self, len: usize, f: impl FnMut() -> T) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorScope::try_alloc_slice_fill_with($access, len, f)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_str`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_str(&$self, src: &str) -> BumpBox<$lifetime, str> {
            BumpAllocatorScope::alloc_str($access, src)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_str`].
        #[inline(always)]
        pub fn try_alloc_str(&$self, src: &str) -> Result<BumpBox<$lifetime, str>, AllocError> {
            BumpAllocatorScope::try_alloc_str($access, src)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_fmt`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_fmt(&$self, args: fmt::Arguments) -> BumpBox<$lifetime, str> {
            BumpAllocatorScope::alloc_fmt($access, args)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_fmt`].
        #[inline(always)]
        pub fn try_alloc_fmt(&$self, args: fmt::Arguments) -> Result<BumpBox<$lifetime, str>, AllocError> {
            BumpAllocatorScope::try_alloc_fmt($access, args)
        }

        /// Forwards to [`MutBumpAllocatorScope::alloc_fmt_mut`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_fmt_mut(&mut $self, args: fmt::Arguments) -> BumpBox<$lifetime, str> {
            MutBumpAllocatorScope::alloc_fmt_mut($access_mut, args)
        }

        /// Forwards to [`MutBumpAllocatorScope::try_alloc_fmt_mut`].
        #[inline(always)]
        pub fn try_alloc_fmt_mut(&mut $self, args: fmt::Arguments) -> Result<BumpBox<$lifetime, str>, AllocError> {
            MutBumpAllocatorScope::try_alloc_fmt_mut($access_mut, args)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_cstr`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr(&$self, src: &CStr) -> &$lifetime CStr {
            BumpAllocatorScope::alloc_cstr($access, src)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_cstr`].
        #[inline(always)]
        pub fn try_alloc_cstr(&$self, src: &CStr) -> Result<&$lifetime CStr, AllocError> {
            BumpAllocatorScope::try_alloc_cstr($access, src)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_cstr_from_str`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr_from_str(&$self, src: &str) -> &$lifetime CStr {
            BumpAllocatorScope::alloc_cstr_from_str($access, src)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_cstr_from_str`].
        #[inline(always)]
        pub fn try_alloc_cstr_from_str(&$self, src: &str) -> Result<&$lifetime CStr, AllocError> {
            BumpAllocatorScope::try_alloc_cstr_from_str($access, src)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_cstr_fmt`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr_fmt(&$self, args: fmt::Arguments) -> &$lifetime CStr {
            BumpAllocatorScope::alloc_cstr_fmt($access, args)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_cstr_fmt`].
        #[inline(always)]
        pub fn try_alloc_cstr_fmt(&$self, args: fmt::Arguments) -> Result<&$lifetime CStr, AllocError> {
            BumpAllocatorScope::try_alloc_cstr_fmt($access, args)
        }

        /// Forwards to [`MutBumpAllocatorScope::alloc_cstr_fmt_mut`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_cstr_fmt_mut(&mut $self, args: fmt::Arguments) -> &$lifetime CStr {
            MutBumpAllocatorScope::alloc_cstr_fmt_mut($access_mut, args)
        }

        /// Forwards to [`MutBumpAllocatorScope::try_alloc_cstr_fmt_mut`].
        #[inline(always)]
        pub fn try_alloc_cstr_fmt_mut(&mut $self, args: fmt::Arguments) -> Result<&$lifetime CStr, AllocError> {
            MutBumpAllocatorScope::try_alloc_cstr_fmt_mut($access_mut, args)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_iter`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter<T>(&$self, iter: impl IntoIterator<Item = T>) -> BumpBox<$lifetime, [T]> {
            BumpAllocatorScope::alloc_iter($access, iter)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_iter`].
        #[inline(always)]
        pub fn try_alloc_iter<T>(&$self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            BumpAllocatorScope::try_alloc_iter($access, iter)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_iter_exact`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter_exact<T, I>(&$self, iter: impl IntoIterator<Item = T, IntoIter = I>) -> BumpBox<$lifetime, [T]>
        where
            I: ExactSizeIterator<Item = T>,
        {
            BumpAllocatorScope::alloc_iter_exact($access, iter)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_iter_exact`].
        #[inline(always)]
        pub fn try_alloc_iter_exact<T, I>(
            &$self,
            iter: impl IntoIterator<Item = T, IntoIter = I>,
        ) -> Result<BumpBox<$lifetime, [T]>, AllocError>
        where
            I: ExactSizeIterator<Item = T>,
        {
            BumpAllocatorScope::try_alloc_iter_exact($access, iter)
        }

        /// Forwards to [`MutBumpAllocatorScope::alloc_iter_mut`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter_mut<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> BumpBox<$lifetime, [T]> {
            MutBumpAllocatorScope::alloc_iter_mut($access_mut, iter)
        }

        /// Forwards to [`MutBumpAllocatorScope::try_alloc_iter_mut`].
        #[inline(always)]
        pub fn try_alloc_iter_mut<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            MutBumpAllocatorScope::try_alloc_iter_mut($access_mut, iter)
        }

        /// Forwards to [`MutBumpAllocatorScope::alloc_iter_mut_rev`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_iter_mut_rev<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> BumpBox<$lifetime, [T]> {
            MutBumpAllocatorScope::alloc_iter_mut_rev($access_mut, iter)
        }

        /// Forwards to [`MutBumpAllocatorScope::try_alloc_iter_mut_rev`].
        #[inline(always)]
        pub fn try_alloc_iter_mut_rev<T>(&mut $self, iter: impl IntoIterator<Item = T>) -> Result<BumpBox<$lifetime, [T]>, AllocError> {
            MutBumpAllocatorScope::try_alloc_iter_mut_rev($access_mut, iter)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_uninit`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_uninit<T>(&$self) -> BumpBox<$lifetime, MaybeUninit<T>> {
            BumpAllocatorScope::alloc_uninit($access)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_uninit`].
        #[inline(always)]
        pub fn try_alloc_uninit<T>(&$self) -> Result<BumpBox<$lifetime, MaybeUninit<T>>, AllocError> {
            BumpAllocatorScope::try_alloc_uninit($access)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_uninit_slice`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_uninit_slice<T>(&$self, len: usize) -> BumpBox<$lifetime, [MaybeUninit<T>]> {
            BumpAllocatorScope::alloc_uninit_slice($access, len)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_uninit_slice`].
        #[inline(always)]
        pub fn try_alloc_uninit_slice<T>(&$self, len: usize) -> Result<BumpBox<$lifetime, [MaybeUninit<T>]>, AllocError> {
            BumpAllocatorScope::try_alloc_uninit_slice($access, len)
        }

        /// Forwards to [`BumpAllocatorScope::alloc_uninit_slice_for`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn alloc_uninit_slice_for<T>(&$self, slice: &[T]) -> BumpBox<$lifetime, [MaybeUninit<T>]> {
            BumpAllocatorScope::alloc_uninit_slice_for($access, slice)
        }

        /// Forwards to [`BumpAllocatorScope::try_alloc_uninit_slice_for`].
        #[inline(always)]
        pub fn try_alloc_uninit_slice_for<T>(&$self, slice: &[T]) -> Result<BumpBox<$lifetime, [MaybeUninit<T>]>, AllocError> {
            BumpAllocatorScope::try_alloc_uninit_slice_for($access, slice)
        }

        /// Forwards to [`BumpAllocator::dealloc`].
        #[inline(always)]
        pub fn dealloc<T: ?Sized>(&$self, boxed: BumpBox<T>) {
            BumpAllocator::dealloc($access, boxed);
        }

        /// Forwards to [`BumpAllocator::reserve`].
        #[inline(always)]
        #[cfg(feature = "panic-on-alloc")]
        pub fn reserve(&$self, additional: usize) {
            BumpAllocator::reserve($access, additional);
        }

        /// Forwards to [`BumpAllocator::try_reserve`].
        #[inline(always)]
        pub fn try_reserve(&$self, additional: usize) -> Result<(), AllocError> {
            BumpAllocator::try_reserve($access, additional)
        }
    };
}

pub(crate) use forward_methods;
