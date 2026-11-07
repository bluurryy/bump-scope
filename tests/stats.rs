#![cfg(all(feature = "std", feature = "panic-on-alloc",))]

mod common;

use bump_scope::Bump;

use common::up_and_down;

up_and_down! {
    const C: BumpConfig = BumpConfigBuilder(DEFAULT_C).const_constructible(true).build();

    #[test]
    fn unallocated() {
        let bump = Bump::<C>::const_new();
        assert_eq!(bump.stats().count(), 0);
        assert_eq!(bump.stats().size(), 0);
        assert_eq!(bump.stats().capacity(), 0);
        assert_eq!(bump.allocated(), 0);
        assert_eq!(bump.stats().remaining(), 0);
    }
}
