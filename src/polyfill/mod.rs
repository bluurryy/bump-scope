//! A collection of stuff that is missing from stable std.
//! Most of this is from nightly.

pub mod iter;
pub mod nonnull;
pub mod nonzero;
pub mod pointer;
pub mod slice;
pub mod usize;

#[cfg(test)]
pub mod layout;

#[allow(dead_code)]
mod other {
    #[inline(always)]
    pub const fn const_unwrap<T: Copy>(option: Option<T>) -> T {
        match option {
            Some(value) => value,
            None => panic!("expected Some"),
        }
    }

    #[cold]
    #[inline(always)]
    pub fn cold() {}

    #[inline(always)]
    pub fn likely(condition: bool) -> bool {
        if condition {
            // ...
        } else {
            cold();
        }

        condition
    }

    #[inline(always)]
    pub fn unlikely(condition: bool) -> bool {
        if condition {
            cold();
        } else {
            // ...
        }

        condition
    }
}

pub use other::*;

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
