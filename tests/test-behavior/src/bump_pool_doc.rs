#[test]
#[cfg(not(miri))] // rayon violates strict-provenance :(
fn test_rayon() {
    use bump_scope::{BumpAllocator, BumpPool};
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    let mut pool: BumpPool = BumpPool::new();

    let strings: Vec<&str> = (0..1000)
        .into_par_iter()
        .map_init(
            || pool.get(),
            |bump, i| {
                // do some expensive work
                bump.alloc_fmt(format_args!("{i}")).into_ref()
            },
        )
        .collect();

    dbg!(&strings);

    pool.reset();

    // memory of the strings is freed, trying to access `strings` will result in a lifetime error
    // dbg!(&strings);
}
