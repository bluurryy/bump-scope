use crate::common::up_and_down;
use bump_scope::Bump;

up_and_down! {
    const C: BumpConfig = DEFAULT_C;

    #[test]
    fn test_chunk_size_4096() {
        const MY_C: BumpConfig = BumpConfigBuilder(C).default_initial_chunk_size(4096).build();
        type MyBump = Bump<MY_C>;

        assert_eq!(0, MyBump::const_new().stats().size());
        assert_eq!(size_of::<[usize; 4]>(), MyBump::with_size(0).stats().size());
        assert_eq!(size_of::<[usize; 4]>(), MyBump::with_size(1).stats().size());
        assert_eq!(size_of::<[usize; 8]>(), MyBump::with_capacity(std::alloc::Layout::new::<u8>()).stats().size());
        assert_eq!(4096, MyBump::with_size(4096).stats().size());
        assert_eq!(4096, MyBump::new().scope_guard().scope().by_value().stats().size());
    }
}
