use core::{hint::unreachable_unchecked, ops::Deref, ptr::NonNull};

pub(crate) enum BaseAllocatorStorageEnum<A> {
    Owned(A),
    Ref(NonNull<A>),
}

pub(crate) struct BaseAllocatorStorage<A> {
    inner: BaseAllocatorStorageEnum<A>,
}

impl<A> BaseAllocatorStorage<A> {
    pub(crate) const fn new(alloc: A) -> Self {
        Self {
            inner: BaseAllocatorStorageEnum::Owned(alloc),
        }
    }

    /// # Safety
    /// This value must have been created by [`new`], not by [`create_ref`].
    ///
    /// [`new`]: BaseAllocatorStorage::new
    /// [`create_ref`]: BaseAllocatorStorage::create_ref
    pub(crate) unsafe fn into_inner(self) -> A {
        match self.inner {
            BaseAllocatorStorageEnum::Owned(a) => a,
            // SAFETY: callee must hold up the safety conditions
            BaseAllocatorStorageEnum::Ref(_) => unsafe { unreachable_unchecked() },
        }
    }
}

impl<A> BaseAllocatorStorage<A> {
    pub(crate) fn create_ref(&self) -> BaseAllocatorStorage<A> {
        let r: &A = self;

        BaseAllocatorStorage {
            inner: BaseAllocatorStorageEnum::Ref(NonNull::from_ref(r)),
        }
    }
}

impl<A> Deref for BaseAllocatorStorage<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            BaseAllocatorStorageEnum::Owned(a) => a,
            // SAFETY: when constructing this variant
            BaseAllocatorStorageEnum::Ref(a) => unsafe { a.as_ref() },
        }
    }
}

impl<A: Default> Default for BaseAllocatorStorage<A> {
    fn default() -> Self {
        Self {
            inner: BaseAllocatorStorageEnum::Owned(A::default()),
        }
    }
}
