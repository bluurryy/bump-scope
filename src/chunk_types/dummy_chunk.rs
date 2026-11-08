//! NB: we mustn't compare the `CLAIMED` or `CONST_CONSTRUCTED` directly, since there might be multiple instances of the `Header` because
//! a `const` can be evaluated multiple times. We must compare the `pos` to `CLAIMED_POS` etc. to find out whether it's a claimed chunk.

use core::ptr::NonNull;

use crate::{
    chunk_types::dummy_memory_region::{CLAIMED_POS, CONST_CONSTRUCTED_POS},
    config::{BumpConfig, config},
};

#[repr(C, align(16))]
struct Header {
    pos: NonNull<u8>,
    end: NonNull<u8>,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct DummyChunk<const C: BumpConfig> {
    header: NonNull<Header>,
}

impl<const C: BumpConfig> DummyChunk<C> {
    pub(crate) const CLAIMED: Self = {
        let static_header: &'static Header = const {
            &Header {
                pos: CLAIMED_POS,
                end: unsafe { CLAIMED_POS.byte_offset(if config!(upwards) { -16 } else { 16 }) },
            }
        };

        Self {
            header: NonNull::from_ref(static_header),
        }
    };

    pub(crate) const CONST_CONSTRUCTED: Self = {
        let static_header: &'static Header = const {
            &Header {
                pos: CONST_CONSTRUCTED_POS,
                end: unsafe { CONST_CONSTRUCTED_POS.byte_offset(if config!(upwards) { -16 } else { 16 }) },
            }
        };

        Self {
            header: NonNull::from_ref(static_header),
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
