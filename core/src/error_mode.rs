use core::alloc::{AllocError, Layout};

/// The error mode for allocations.
///
/// This is either [`AllocError`] or [`!`](prim@never). When choosing `!`,
/// the allocation method will panic on allocation failure instead of returning an error.
///
/// # Examples
///
/// Infallible aka panicking allocation.
/// ```
/// # extern crate alloc;
/// use alloc::alloc::{Global, Layout};
/// use bump_scope_core::config::{BumpConfig, BumpConfigBuilder, DEFAULT_BUMP_CONFIG};
///
/// type Bump = bump_scope_core::Bump<DEFAULT_BUMP_CONFIG, Global>;
///
/// let bump = Bump::const_new_in(Global);
/// let layout = Layout::new::<u8>();
///
/// let ptr = match bump.alloc::<!>(layout) {
///     Ok(ok) => ok,
/// };
///
/// // in rust version 1.101.0 you'll be able to write `into_ok`
/// #[cfg(feature = "nightly")]
/// let ptr = bump.alloc::<!>(layout).into_ok();
/// ```
#[expect(private_bounds)]
pub trait ErrorMode: Sealed {}
impl ErrorMode for AllocError {}
impl ErrorMode for ! {}

pub(crate) trait Sealed {
    fn capacity_overflow() -> Self;
    fn allocation(layout: Layout) -> Self;
    fn claimed() -> Self;
}

impl Sealed for AllocError {
    #[inline(always)]
    fn capacity_overflow() -> Self {
        Self
    }

    #[inline(always)]
    fn allocation(_: Layout) -> Self {
        Self
    }

    #[inline(always)]
    fn claimed() -> Self {
        Self
    }
}

impl Sealed for ! {
    #[cold]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    fn capacity_overflow() -> Self {
        panic!("capacity overflow");
    }

    #[cold]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    fn allocation(layout: Layout) -> Self {
        cfg_select! {
            feature = "alloc" => alloc::alloc::handle_alloc_error(layout),
            _ => {
                _ = layout;
                panic!("allocation failed")
            }
        }
    }

    #[cold]
    #[cfg_attr(not(panic = "immediate-abort"), inline(never))]
    fn claimed() -> Self {
        panic!("bump allocator is claimed");
    }
}
