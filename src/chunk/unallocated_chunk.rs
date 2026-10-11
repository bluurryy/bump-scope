use core::{marker::PhantomData, ptr::NonNull};

use crate::{chunk::UnallocatedChunkHeader, error_behavior::ErrorBehavior, settings::BumpAllocatorSettings};

/// Represents **an unallocated** chunk.
///
/// This chunk is used when creating a `Bump` with `new`. This creates a bump allocator
/// with a header storing the default constructor for the base allocator type.
pub(crate) struct UnallocatedChunk<A, S> {
    pub(crate) header: NonNull<UnallocatedChunkHeader<A>>,
    pub(crate) marker: PhantomData<fn() -> (A, S)>,
}

impl<A, S> UnallocatedChunk<A, S>
where
    S: BumpAllocatorSettings,
{
    #[cfg_attr(not(feature = "std"), expect(clippy::unnecessary_wraps))]
    pub(crate) fn construct_base_allocator<E: ErrorBehavior>(self) -> Result<A, E> {
        // We must handle unwinding when calling `default`, because this function
        // is called in allocator api routines, which don't allow unwinding.
        let default = unsafe { self.header.as_ref() }.default;

        #[cfg(feature = "std")]
        {
            match std::panic::catch_unwind(default) {
                Ok(ok) => Ok(ok),
                Err(_) => Err(E::base_allocator_default_constructor_panicked()),
            }
        }

        #[cfg(not(feature = "std"))]
        {
            Ok(crate::polyfill::panic::abort_on_unwind(default))
        }
    }
}
