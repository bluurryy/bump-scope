#![cfg(all(feature = "std", feature = "panic-on-alloc"))]

mod common;

use std::alloc::Global;

use bump_scope::{Bump, BumpString};

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn shrinks() {
        let bump: Bump<Global, C> = Bump::new();
        let mut string = BumpString::from_str_in("1234", &bump);
        assert_eq!(bump.stats().allocated(), 4);
        string.pop();
        string.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 3);
        string.clear();
        string.shrink_to_fit();
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn deallocates() {
        let bump: Bump<Global, C> = Bump::new();
        let string = BumpString::from_str_in("123", &bump);
        assert_eq!(bump.stats().allocated(), 3);
        drop(string);
        assert_eq!(bump.stats().allocated(), 0);
    }

    #[test]
    fn into_str() {
        let bump: Bump<Global, C> = Bump::new();
        let mut string = BumpString::from_str_in("12345", &bump);
        assert_eq!(bump.stats().allocated(), 5);
        string.truncate(3);
        let slice = string.into_str();
        assert_eq!(bump.stats().allocated(), 3);
        _ = slice;
    }

    #[test]
    fn into_str_without_shrink() {
        let bump: Bump<Global, C> = Bump::new();
        let mut string = BumpString::from_str_in("12345", &bump);
        assert_eq!(bump.stats().allocated(), 5);
        string.truncate(3);
        let slice = string.into_fixed_string().into_str();
        assert_eq!(bump.stats().allocated(), 5);
        _ = slice;
    }
}
