use core::{hint::unreachable_unchecked, ops::Deref, ptr::NonNull};

pub(crate) enum BaseAllocatorStorageEnum<A> {
    Owned(A),
    Ref(NonNull<A>),
}

use BaseAllocatorStorageEnum::{Owned, Ref};

pub(crate) struct BaseAllocatorStorage<A> {
    inner: BaseAllocatorStorageEnum<A>,
}

impl<A> BaseAllocatorStorage<A> {
    pub(crate) const fn new(alloc: A) -> Self {
        Self { inner: Owned(alloc) }
    }

    /// # Safety
    /// This value must have been created by [`new`], not by [`create_ref`].
    ///
    /// [`new`]: BaseAllocatorStorage::new
    /// [`create_ref`]: BaseAllocatorStorage::create_ref
    pub(crate) unsafe fn into_inner(self) -> A {
        match self.inner {
            Owned(a) => a,
            // SAFETY: callee must hold up the safety conditions
            Ref(_) => unsafe { unreachable_unchecked() },
        }
    }
}

impl<A> BaseAllocatorStorage<A> {
    pub(crate) fn create_ref(&self) -> BaseAllocatorStorage<A> {
        let r: &A = self;

        BaseAllocatorStorage {
            inner: Ref(NonNull::from_ref(r)),
        }
    }
}

impl<A> Deref for BaseAllocatorStorage<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        match &self.inner {
            Owned(a) => a,
            Ref(a) => unsafe { a.as_ref() },
        }
    }
}
