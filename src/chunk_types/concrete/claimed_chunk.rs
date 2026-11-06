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
pub(crate) struct ClaimedChunk<const C: BumpConfig, A> {
    header: NonNull<Header>,
    marker: PhantomData<*const A>,
}

impl<const C: BumpConfig, A> Clone for ClaimedChunk<C, A> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<const C: BumpConfig, A> Copy for ClaimedChunk<C, A> {}

impl<const C: BumpConfig, A> ClaimedChunk<C, A> {
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
