use core::{marker::PhantomData, ptr::NonNull};

use crate::{
    chunk_types::dummy_memory_region::CLAIMED_POS as POS,
    config::{BumpConfig, config},
};

#[repr(C, align(16))]
struct Header {
    pos: NonNull<u8>,
    end: NonNull<u8>,
}

#[repr(transparent)]
pub(crate) struct ClaimedChunk<A, const C: BumpConfig> {
    header: NonNull<Header>,
    marker: PhantomData<*const A>,
}

impl<A, const C: BumpConfig> Clone for ClaimedChunk<A, C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, const C: BumpConfig> Copy for ClaimedChunk<A, C> {}

impl<A, const C: BumpConfig> ClaimedChunk<A, C> {
    pub(crate) const VALUE: Self = {
        let static_header: &'static Header = const {
            &Header {
                pos: POS,
                end: unsafe { POS.byte_offset(if config!(upwards) { -16 } else { 16 }) },
            }
        };

        Self {
            header: NonNull::from_ref(static_header).cast(),
            marker: PhantomData,
        }
    };

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
}
