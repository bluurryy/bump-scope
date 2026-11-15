use core::{clone::TrivialClone, ops::Deref};

use alloc::alloc::AllocatorClone;

pub(crate) struct BaseAllocatorStorage<A> {
    inner: <A as BaseAllocatorStorageType>::Storage,
}

impl<A> BaseAllocatorStorage<A> {
    pub(crate) const fn new(alloc: A) -> Self {
        Self {
            inner: <<A as BaseAllocatorStorageType>::Storage>::new(alloc),
        }
    }

    /// # Safety
    /// This value must have been created by [`new`], not by [`create_ref`].
    ///
    /// [`new`]: BaseAllocatorStorage::new
    /// [`create_ref`]: BaseAllocatorStorage::create_ref
    pub(crate) unsafe fn into_inner(self) -> A {
        unsafe { self.inner.into_inner() }
    }

    pub(crate) fn create_ref(&self) -> BaseAllocatorStorage<A> {
        BaseAllocatorStorage {
            inner: self.inner.create_ref(),
        }
    }
}

impl<A> Deref for BaseAllocatorStorage<A> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

trait BaseAllocatorStorageType: Sized {
    type Storage: const BaseAllocatorStorageNew<Self> + BaseAllocatorStorageMethods<Self> + Deref<Target = Self>;
}

impl<A> BaseAllocatorStorageType for A {
    default type Storage = default::DefaultBaseAllocatorStorage<Self>;
}

// Only use the cloning storage when a clone is trivial, like for `Global`, `System`, `&A`, ...
impl<A: AllocatorClone + TrivialClone> BaseAllocatorStorageType for A {
    type Storage = clone::CloneBaseAllocatorStorage<Self>;
}

const trait BaseAllocatorStorageNew<A> {
    fn new(alloc: A) -> Self;
}

trait BaseAllocatorStorageMethods<A> {
    fn create_ref(&self) -> Self;

    /// See [`BaseAllocatorStorage::into_inner`].
    unsafe fn into_inner(self) -> A;
}

mod default {
    use core::{hint::unreachable_unchecked, ops::Deref, ptr::NonNull};

    use crate::base_allocator_storage::nightly::{BaseAllocatorStorageMethods, BaseAllocatorStorageNew};

    pub(super) enum DefaultBaseAllocatorStorage<A> {
        Owned(A),
        Ref(NonNull<A>),
    }

    use DefaultBaseAllocatorStorage::{Owned, Ref};

    const impl<A> BaseAllocatorStorageNew<A> for DefaultBaseAllocatorStorage<A> {
        fn new(alloc: A) -> Self {
            Owned(alloc)
        }
    }

    impl<A> BaseAllocatorStorageMethods<A> for DefaultBaseAllocatorStorage<A> {
        fn create_ref(&self) -> Self {
            Ref(NonNull::from_ref(self))
        }

        unsafe fn into_inner(self) -> A {
            match self {
                Owned(a) => a,
                // SAFETY: callee must hold up the safety conditions
                Ref(_) => unsafe { unreachable_unchecked() },
            }
        }
    }

    impl<A> Deref for DefaultBaseAllocatorStorage<A> {
        type Target = A;

        fn deref(&self) -> &Self::Target {
            match self {
                Owned(a) => a,
                Ref(a) => unsafe { a.as_ref() },
            }
        }
    }
}

mod clone {
    use core::ops::Deref;

    use alloc::alloc::AllocatorClone;

    use crate::base_allocator_storage::nightly::{BaseAllocatorStorageMethods, BaseAllocatorStorageNew};

    pub(super) struct CloneBaseAllocatorStorage<A> {
        alloc: A,
    }

    const impl<A> BaseAllocatorStorageNew<A> for CloneBaseAllocatorStorage<A> {
        fn new(alloc: A) -> Self {
            Self { alloc }
        }
    }

    impl<A: AllocatorClone> BaseAllocatorStorageMethods<A> for CloneBaseAllocatorStorage<A> {
        fn create_ref(&self) -> Self {
            Self { alloc: (**self).clone() }
        }

        unsafe fn into_inner(self) -> A {
            self.alloc
        }
    }

    impl<A> Deref for CloneBaseAllocatorStorage<A> {
        type Target = A;

        fn deref(&self) -> &Self::Target {
            &self.alloc
        }
    }
}
