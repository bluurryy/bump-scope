use bump_scope::Bump;

use crate::common::up_and_down;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

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
