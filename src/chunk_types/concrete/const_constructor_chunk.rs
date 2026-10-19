use core::{marker::PhantomData, ptr::NonNull};

use crate::{
    chunk_types::dummy_memory_region::CONST_CONSTRUCTOR_POS as POS,
    config::{BumpConfig, config},
    error_behavior::ErrorBehavior,
};

#[repr(C, align(16))]
struct Header<A> {
    pos: NonNull<u8>,
    end: NonNull<u8>,
    default: fn() -> A,
}

#[repr(transparent)]
pub(crate) struct ConstConstructorChunk<A, const C: BumpConfig> {
    header: NonNull<Header<A>>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for ConstConstructorChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for ConstConstructorChunk<A, C> {}

impl<A, const C: BumpConfig> ConstConstructorChunk<A, C>
where
    A: Default + 'static,
{
    pub(crate) const VALUE: Self = {
        let static_header: &'static Header<A> = const {
            &Header {
                pos: POS,
                end: unsafe { POS.byte_offset(if config!(upwards) { -16 } else { 16 }) },
                default: A::default,
            }
        };

        Self {
            header: NonNull::from_ref(static_header).cast(),
            marker: PhantomData,
        }
    };
}

impl<A, const C: BumpConfig> ConstConstructorChunk<A, C> {
    #[expect(dead_code)]
    pub(crate) fn as_ptr(self) -> NonNull<()> {
        self.header.cast()
    }

    #[expect(dead_code)]
    pub(crate) fn pos(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.pos
    }

    #[expect(dead_code)]
    pub(crate) fn end(self) -> NonNull<u8> {
        unsafe { self.header.as_ref() }.end
    }

    pub(crate) fn default(self) -> fn() -> A {
        unsafe { self.header.as_ref() }.default
    }

    #[cfg_attr(not(feature = "std"), expect(clippy::unnecessary_wraps))]
    pub(crate) fn construct_base_allocator<E: ErrorBehavior>(self) -> Result<A, E> {
        // We must handle unwinding when calling `default`, because this function
        // is called in allocator api routines, which don't allow unwinding.
        let default = self.default();

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
