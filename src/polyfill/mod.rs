//! A collection of stuff that is missing from stable std.
//! Most of this is from nightly.

pub(crate) mod iter;
pub(crate) mod nonnull;
pub(crate) mod nonzero;
pub(crate) mod pointer;
pub(crate) mod slice;
pub(crate) mod str;
pub(crate) mod usize;

#[allow(dead_code)]
mod other {
    #[inline(always)]
    pub(crate) const fn const_unwrap<T: Copy>(option: Option<T>) -> T {
        match option {
            Some(value) => value,
            None => panic!("expected Some"),
        }
    }

    #[cold]
    #[inline(always)]
    pub(crate) fn cold() {}

    #[inline(always)]
    pub(crate) fn likely(condition: bool) -> bool {
        if condition {
            // ...
        } else {
            cold();
        }

        condition
    }

    #[inline(always)]
    pub(crate) fn unlikely(condition: bool) -> bool {
        if condition {
            cold();
        } else {
            // ...
        }

        condition
    }
}

use core::mem::ManuallyDrop;

pub(crate) use other::*;

macro_rules! cfg_const {
    (
        #[cfg_const($($tt:tt)*)]
        $(#[$attr:meta])*
        $vis:vis fn $ident:ident($($params:tt)*) $(-> $result:ty)? $body:block
    ) => {
        #[cfg($($tt)*)]
        $(#[$attr])*
        $vis const fn $ident($($params)*) $(-> $result)? $body

        #[cfg(not($($tt)*))]
        $(#[$attr])*
        $vis fn $ident($($params)*) $(-> $result)? $body
    };
}

pub(crate) use cfg_const;

pub(crate) const unsafe fn transmute_value<A, B>(a: A) -> B {
    assert!(size_of::<A>() == size_of::<B>());
    core::mem::transmute_copy(&ManuallyDrop::new(a))
}

pub(crate) const unsafe fn transmute_ref<A, B>(a: &A) -> &B {
    assert!(size_of::<A>() == size_of::<B>());
    &*(a as *const A).cast::<B>()
}

pub(crate) unsafe fn transmute_mut<A, B>(a: &mut A) -> &mut B {
    assert!(size_of::<A>() == size_of::<B>());
    &mut *(a as *mut A).cast::<B>()
}
