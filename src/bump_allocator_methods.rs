use core::{
    alloc::Layout,
    ffi::CStr,
    fmt,
    mem::MaybeUninit,
    ptr::{self, NonNull},
    slice,
};

use crate::{
    panic_on_error, polyfill::nonnull, BumpAllocatorScope, BumpBox, BumpString, BumpVec, FixedBumpString, FixedBumpVec,
    MutBumpAllocator, MutBumpString, MutBumpVec, MutBumpVecRev, SizedTypeProperties,
};

/// TODO
pub trait BumpAllocatorMethods<'a>: BumpAllocatorScope<'a> + Sized {
    /// Allocate an object.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc(123);
    /// assert_eq!(allocated, 123);
    /// ```
    fn alloc<T>(self, value: T) -> BumpBox<'a, T> {
        self.alloc_uninit().init(value)
    }

    /// Pre-allocate space for an object. Once space is allocated `f` will be called to create the value to be put at that place.
    /// In some situations this can help the compiler realize that `T` can be constructed at the allocated space instead of having to copy it over.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_with(|| 123);
    /// assert_eq!(allocated, 123);
    /// ```
    fn alloc_with<T>(self, f: impl FnOnce() -> T) -> BumpBox<'a, T> {
        self.alloc_uninit().init(f())
    }

    /// Allocate an object with its default value.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_default::<i32>();
    /// assert_eq!(allocated, 0);
    /// ```
    fn alloc_default<T: Default>(self) -> BumpBox<'a, T> {
        self.alloc_uninit().init(Default::default())
    }

    /// Allocate a slice and `Copy` elements from an existing slice.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_copy(&[1, 2, 3]);
    /// assert_eq!(allocated, [1, 2, 3]);
    /// ```
    fn alloc_slice_copy<T: Copy>(self, slice: &[T]) -> BumpBox<'a, [T]> {
        self.alloc_uninit_slice_for(slice).init_copy(slice)
    }

    /// Allocate a slice and `Clone` elements from an existing slice.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_clone(&[String::from("a"), String::from("b")]);
    /// assert_eq!(allocated, [String::from("a"), String::from("b")]);
    /// ```
    fn alloc_slice_clone<T: Clone>(self, slice: &[T]) -> BumpBox<'a, [T]> {
        self.alloc_uninit_slice_for(slice).init_clone(slice)
    }

    /// Allocate a slice and fill it with elements by cloning `value`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill(3, "ho");
    /// assert_eq!(allocated, ["ho", "ho", "ho"]);
    /// ```
    fn alloc_slice_fill<T: Clone>(self, len: usize, value: T) -> BumpBox<'a, [T]> {
        self.alloc_uninit_slice(len).init_fill(value)
    }

    /// Allocates a slice by fill it with elements returned by calling a closure repeatedly.
    ///
    /// This method uses a closure to create new values. If you'd rather
    /// [`Clone`] a given value, use [`alloc_slice_fill`](Self::alloc_slice_fill). If you want to use the [`Default`]
    /// trait to generate values, you can pass [`Default::default`] as the
    /// argument.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_slice_fill_with::<i32>(3, Default::default);
    /// assert_eq!(allocated, [0, 0, 0]);
    /// ```
    fn alloc_slice_fill_with<T>(self, len: usize, f: impl FnMut() -> T) -> BumpBox<'a, [T]> {
        self.alloc_uninit_slice(len).init_fill_with(f)
    }

    /// Allocate a `str`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_str("Hello world!");
    /// assert_eq!(allocated, "Hello world!");
    /// ```
    fn alloc_str(self, src: &str) -> BumpBox<'a, str> {
        let bytes = self.alloc_slice_copy(src.as_bytes());
        unsafe { BumpBox::from_utf8_unchecked(bytes) }
    }

    /// Allocate a `str` from format arguments.
    ///
    /// If you have a `&mut self` you can use [`alloc_fmt_mut`](Self::alloc_fmt_mut) instead for better performance.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// Panics if a formatting trait implementation returned an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    fn alloc_fmt(self, args: fmt::Arguments) -> BumpBox<'a, str> {
        if let Some(string) = args.as_str() {
            return self.alloc_str(string);
        }

        let mut string = BumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_boxed_str()
    }

    /// Allocate a `str` from format arguments.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`](Self::alloc_fmt). By taking `self` as `&mut`, it can use the entire remaining chunk space
    /// as the capacity for its string buffer. As a result, the string buffer rarely needs to grow.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// Panics if a formatting trait implementation returned an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    ///
    /// assert_eq!(string, "1 + 2 = 3");
    /// ```
    fn alloc_fmt_mut(self, args: fmt::Arguments) -> BumpBox<'a, str>
    where
        Self: MutBumpAllocator,
    {
        if let Some(string) = args.as_str() {
            return self.alloc_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_boxed_str()
    }

    /// Allocate a `CStr`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr(c"Hello world!");
    /// assert_eq!(allocated, c"Hello world!");
    /// ```
    fn alloc_cstr(self, src: &CStr) -> &'a CStr {
        let bytes = self.alloc_slice_copy(src.to_bytes_with_nul()).into_ref();
        unsafe { CStr::from_bytes_with_nul_unchecked(bytes) }
    }

    /// Allocate a `CStr` from a `str`.
    ///
    /// If `src` contains a `'\0'` then the `CStr` will stop there.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let allocated = bump.alloc_cstr_from_str("Hello world!");
    /// assert_eq!(allocated, c"Hello world!");
    ///
    /// let allocated = bump.alloc_cstr_from_str("abc\0def");
    /// assert_eq!(allocated, c"abc");
    /// ```
    fn alloc_cstr_from_str(self, src: &str) -> &'a CStr {
        let src = src.as_bytes();

        if let Some(nul) = src.iter().position(|&c| c == b'\0') {
            let bytes_with_nul = unsafe { src.get_unchecked(..nul + 1) };
            let cstr = unsafe { CStr::from_bytes_with_nul_unchecked(bytes_with_nul) };
            self.alloc_cstr(cstr)
        } else {
            // `src` contains no null
            let dst = self.allocate_slice(src.len() + 1);

            unsafe {
                ptr::copy_nonoverlapping(src.as_ptr(), dst.as_ptr(), src.len());
                dst.as_ptr().add(src.len()).write(0);

                let bytes = slice::from_raw_parts(dst.as_ptr(), src.len() + 1);
                CStr::from_bytes_with_nul_unchecked(bytes)
            }
        }
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    ///
    /// If you have a `&mut self` you can use [`alloc_cstr_fmt_mut`](Self::alloc_cstr_fmt_mut) instead for better performance.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// Panics if a formatting trait implementation returned an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    fn alloc_cstr_fmt(self, args: fmt::Arguments) -> &'a CStr {
        if let Some(string) = args.as_str() {
            return self.alloc_cstr_from_str(string);
        }

        let mut string = BumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_cstr()
    }

    /// Allocate a `CStr` from format arguments.
    ///
    /// If the string contains a `'\0'` then the `CStr` will stop there.
    ///
    /// This function is designed as a performance improvement over [`alloc_fmt`](Self::alloc_fmt). By taking `self` as `&mut`, it can use the entire remaining chunk space
    /// as the capacity for its string buffer. As a result, the string buffer rarely needs to grow.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// Panics if a formatting trait implementation returned an error.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let one = 1;
    /// let two = 2;
    /// let string = bump.alloc_cstr_fmt_mut(format_args!("{one} + {two} = {}", one + two));
    /// assert_eq!(string, c"1 + 2 = 3");
    ///
    /// let one = bump.alloc_cstr_fmt_mut(format_args!("{one}\0{two}"));
    /// assert_eq!(one, c"1");
    /// ```
    fn alloc_cstr_fmt_mut(self, args: fmt::Arguments) -> &'a CStr
    where
        Self: MutBumpAllocator,
    {
        if let Some(string) = args.as_str() {
            return self.alloc_cstr_from_str(string);
        }

        let mut string = MutBumpString::new_in(self);
        panic_on_error(string.generic_write_fmt(args));
        string.into_cstr()
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// If you have an `impl ExactSizeIterator` then you can use [`alloc_iter_exact`] instead for better performance.
    ///
    /// If `iter` is not an `ExactSizeIterator` but you have a `&mut self` you can still get somewhat better performance by using [`alloc_iter_mut`].
    ///
    /// [`alloc_iter_exact`]: Self::alloc_iter_exact
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    fn alloc_iter<T>(self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]> {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = BumpVec::<T, Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }

    /// Allocate elements of an iterator into a slice.
    ///
    /// This function is designed as a performance improvement over [`alloc_iter`](Self::alloc_iter). By taking `self` as `&mut`, it can use the entire remaining chunk space
    /// as the capacity for its string buffer. As a result, the string buffer rarely needs to grow.
    ///
    /// When bumping downwards, prefer [`alloc_iter_mut_rev`](Bump::alloc_iter_mut_rev) instead.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    fn alloc_iter_mut<T>(self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]>
    where
        Self: MutBumpAllocator,
    {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVec::<T, Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }

    /// Allocate elements of an iterator into a slice in reverse order.
    ///
    /// Compared to [`alloc_iter_mut`] this function is more performant
    /// for downwards bumping allocators as the allocation for the vector can be shrunk in place
    /// without any `ptr::copy`.
    ///
    /// The reverse is true when upwards allocating. In that case it's better to use [`alloc_iter_mut`] to prevent
    /// the `ptr::copy`.
    ///
    /// [`alloc_iter_mut`]: Self::alloc_iter_mut
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_mut_rev([1, 2, 3]);
    /// assert_eq!(slice, [3, 2, 1]);
    /// ```
    fn alloc_iter_mut_rev<T>(self, iter: impl IntoIterator<Item = T>) -> BumpBox<'a, [T]>
    where
        Self: MutBumpAllocator,
    {
        let iter = iter.into_iter();
        let capacity = iter.size_hint().0;

        let mut vec = MutBumpVecRev::<T, Self>::with_capacity_in(capacity, self);

        for value in iter {
            vec.push(value);
        }

        vec.into_boxed_slice()
    }

    /// Allocate elements of an `ExactSizeIterator` into a slice.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let slice = bump.alloc_iter_exact([1, 2, 3]);
    /// assert_eq!(slice, [1, 2, 3]);
    /// ```
    fn alloc_iter_exact<T, I: ExactSizeIterator<Item = T>>(
        self,
        iter: impl IntoIterator<Item = T, IntoIter = I>,
    ) -> BumpBox<'a, [T]> {
        let mut iter = iter.into_iter();
        let len = iter.len();

        let mut vec = BumpVec::<T, Self>::with_capacity_in(len, self);

        while vec.len() != vec.capacity() {
            match iter.next() {
                // SAFETY: we checked above that `len != capacity`, so there is space
                Some(value) => unsafe { vec.unchecked_push(value) },
                None => break,
            }
        }

        // suppressing shrink by going through a fixed vec
        vec.into_fixed_vec().into_boxed_slice()
    }

    /// Allocate an unitialized object.
    ///
    /// You can safely initialize the object with [`init`](BumpBox::init) or unsafely with [`assume_init`](BumpBox::assume_init).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Safely:
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let five = bump.alloc_uninit();
    ///
    /// let five = five.init(5);
    ///
    /// assert_eq!(*five, 5)
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let mut bump: Bump = Bump::new();
    /// let mut five = bump.alloc_uninit();
    ///
    /// let five = unsafe {
    ///     five.write(5);
    ///     five.assume_init()
    /// };
    ///
    /// assert_eq!(*five, 5)
    /// ```
    fn alloc_uninit<T>(self) -> BumpBox<'a, MaybeUninit<T>> {
        if T::IS_ZST {
            return BumpBox::zst(MaybeUninit::uninit());
        }

        let ptr = self.allocate_sized::<MaybeUninit<T>>();
        unsafe { BumpBox::from_raw(ptr) }
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// Safely:
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let values = bump.alloc_uninit_slice(3);
    ///
    /// let values = values.init_copy(&[1, 2, 3]);
    ///
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    ///
    /// Unsafely:
    /// ```
    /// # use bump_scope::{ Bump, BumpAllocatorMethods };
    /// # let bump: Bump = Bump::new();
    /// let mut values = bump.alloc_uninit_slice(3);
    ///
    /// let values = unsafe {
    ///     values[0].write(1);
    ///     values[1].write(2);
    ///     values[2].write(3);
    ///
    ///     values.assume_init()
    /// };
    ///
    /// assert_eq!(values, [1, 2, 3]);
    /// ```
    fn alloc_uninit_slice<T>(self, len: usize) -> BumpBox<'a, [MaybeUninit<T>]> {
        if T::IS_ZST {
            return BumpBox::uninit_zst_slice(len);
        }

        let ptr = self.allocate_slice(len);
        let slice = nonnull::slice_from_raw_parts(ptr, len);
        unsafe { BumpBox::from_raw(slice) }
    }

    /// Allocate an unitialized object slice.
    ///
    /// You can safely initialize the object with
    /// [`init_fill`](BumpBox::init_fill),
    /// [`init_fill_with`](BumpBox::init_fill_with),
    /// [`init_copy`](BumpBox::init_copy),
    /// [`init_clone`](BumpBox::init_clone) or unsafely with
    /// [`assume_init`](BumpBox::assume_init).
    ///
    /// This is just like [`alloc_uninit_slice`](Self::alloc_uninit_slice) but uses a `slice` to provide the `len`.
    /// This avoids a check for a valid layout. The elements of `slice` are irrelevant.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let mut bump: Bump = Bump::new();
    /// let slice = &[1, 2, 3];
    /// let other_slice = bump.alloc_uninit_slice_for(slice);
    /// assert_eq!(other_slice.len(), 3);
    /// ```
    fn alloc_uninit_slice_for<T>(self, slice: &[T]) -> BumpBox<'a, [MaybeUninit<T>]> {
        if T::IS_ZST {
            return BumpBox::uninit_zst_slice(slice.len());
        }

        let ptr = self.allocate_slice_for(slice).cast::<MaybeUninit<T>>();
        let slice = nonnull::slice_from_raw_parts(ptr, slice.len());
        unsafe { BumpBox::from_raw(slice) }
    }

    /// Allocate a [`FixedBumpVec`] with the given `capacity`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut values = bump.alloc_fixed_vec(3);
    /// values.push(1);
    /// values.push(2);
    /// values.push(3);
    /// assert_eq!(values, [1, 2, 3])
    /// ```
    fn alloc_fixed_vec<T>(self, capacity: usize) -> FixedBumpVec<'a, T> {
        let uninit = self.alloc_uninit_slice(capacity);
        FixedBumpVec::from_uninit(uninit)
    }

    /// Allocate a [`FixedBumpString`] with the given `capacity` in bytes.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::Bump;
    /// # let bump: Bump = Bump::new();
    /// let mut string = bump.alloc_fixed_string(12);
    /// string.push_str("Hello");
    /// string.push_str(" world!");
    /// assert_eq!(string, "Hello world!");
    /// ```
    fn alloc_fixed_string(self, capacity: usize) -> FixedBumpString<'a> {
        let uninit = self.alloc_uninit_slice(capacity);
        FixedBumpString::from_uninit(uninit)
    }

    /// Allocates memory as described by the given `Layout`.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    fn alloc_layout(self, layout: Layout) -> NonNull<u8> {
        self.allocate_layout(layout)
    }

    /// Reserves capacity for at least `additional` more bytes to be bump allocated.
    /// The bump allocator may reserve more space to avoid frequent reallocations.
    /// After calling `reserve_bytes`, <code>self.[stats](Self::stats)().[remaining](Stats::remaining)()</code> will be greater than or equal to
    /// `additional`. Does nothing if the capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the allocation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bump_scope::{ Bump };
    /// let bump: Bump = Bump::new();
    /// assert!(bump.stats().capacity() < 4096);
    ///
    /// bump.reserve_bytes(4096);
    /// assert!(bump.stats().capacity() >= 4096);
    /// ```
    fn reserve_bytes(self, additional: usize) {
        _ = additional;
        todo!("TODO") // TODO
    }
}
