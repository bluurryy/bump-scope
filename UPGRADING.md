## [Unreleased]

- If you are bump allocating `hashbrown` types you have to enable the `allocator-api2-02` feature unless you are using `hashbrown`'s `nightly` feature then you only need the `nightly-allocator-api` feature.
- If you are using the methods `alloc_zeroed(_slice)`, `init_zeroed`, `resize_zeroed` or `extend_zeroed` you now have to enable the `zerocopy-08` feature (instead of `zerocopy`) and import the respective extension traits from `bump_scope::zerocopy_08`.
- If you were using a custom base allocator you now have to make it implement this crate's `Allocator` trait. You can safely do that using a wrapper type from the `bump_scope::alloc::compat` module.
- Uses of `bump_format!(in bump, ...` where `bump` is not a reference now have to write `bump_format!(in &bump, ...` for the same behavior
- There are other less impactful breaking changes that you might encounter. [You can read about them in the changelog.](CHANGELOG.md#Unreleased)

[Unreleased]: CHANGELOG.md#Unreleased