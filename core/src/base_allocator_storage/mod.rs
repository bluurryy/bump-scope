cfg_select! {
    feature = "nightly" => {
        mod nightly;
        pub(crate) use nightly::BaseAllocatorStorage;
    }
    _ => {
        mod stable;
        pub(crate) use stable::BaseAllocatorStorage;
    }
}
