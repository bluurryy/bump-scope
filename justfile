export RUST_BACKTRACE := "1"
export MIRIFLAGS := "-Zmiri-strict-provenance -Zmiri-symbolic-alignment-check"

[private]
@default:
  just --list

# Runs checks before a release.
[group('release')]
pre-release:
  typos
  just doc
  just check
  just test
  cargo +stable semver-checks

# Installs all tools required for `pre-release` except for `nu`.
[group('release')]
setup:
  cargo binstall typos-cli@1.42.0 --locked
  cargo binstall cargo-insert-docs@1.3.0 --locked
  cargo binstall cargo-semver-checks@0.45.0 --locked
  cargo binstall cargo-hack@0.6.41 --locked
  cargo binstall cargo-minimal-versions@0.1.35 --locked

# Runs `cargo fmt` on everything.
[group('fmt')]
fmt:
  cargo +nightly fmt
  cd crates/callgrind-benches && cargo +nightly fmt
  cd crates/criterion-benches && cargo +nightly fmt
  cd crates/fuzzing-support && cargo +nightly fmt
  cd crates/test-fallibility && cargo +nightly fmt
  cd crates/test-hashbrown && cargo +nightly fmt
  cd crates/tests-from-std && cargo +nightly fmt
  cd fuzz && cargo +nightly fmt

# Runs all `check-*`.
[group('check')]
check: 
  just check-fmt
  just check-clippy
  just check-msrv
  just check-nostd
  just check-fallibility

# Checks formatting.
[group('check')]
check-fmt:
  cargo +stable fmt --all --check
  cd crates/callgrind-benches && cargo +stable fmt --all --check
  cd crates/criterion-benches && cargo +stable fmt --all --check
  cd crates/fuzzing-support && cargo +stable fmt --all --check
  cd crates/test-fallibility && cargo +stable fmt --all --check
  cd crates/test-hashbrown && cargo +stable fmt --all --check
  cd crates/tests-from-std && cargo +stable fmt --all --check
  cd fuzz && cargo +stable fmt --all --check

# Runs all `check-clippy-*`.
[group('check')]
check-clippy:
  just check-clippy-stable
  just check-clippy-nightly

# Runs clippy on the stable toolchain.
[group('check')]
check-clippy-stable:
  cargo +stable clippy --tests --no-default-features -- -Dwarnings
  cargo +stable clippy --tests --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde -- -Dwarnings

# Runs clippy on the nightly toolchain.
[group('check')]
check-clippy-nightly:
  cargo +nightly clippy --tests --no-default-features -- -Dwarnings
  cargo +nightly clippy --tests --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde -- -Dwarnings
  cargo +nightly clippy --tests --all-features -- -Dwarnings
  cd crates/callgrind-benches && cargo +nightly clippy --tests --benches --workspace -- -Dwarnings
  cd crates/criterion-benches && cargo +nightly clippy --tests --benches --workspace -- -Dwarnings
  cd crates/fuzzing-support && cargo +nightly clippy --tests -- -Dwarnings
  cd crates/test-fallibility && cargo +nightly clippy --tests -- -Dwarnings
  cd crates/test-hashbrown && cargo +nightly clippy --tests -- -Dwarnings
  cd crates/test-hashbrown && cargo +nightly clippy --tests --all-features -- -Dwarnings
  cd crates/tests-from-std && cargo +nightly clippy --tests -- -Dwarnings
  cd fuzz && cargo +nightly clippy -- -Dwarnings

# Runs `cargo check` with the minimum supported rust version.
[group('check')]
check-msrv:
  # msrv might print warnings that stable doesnt, we dont care
  cargo +1.85.1 check --no-default-features
  cargo +1.85.1 check --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde

# Runs `cargo check` with mininmal dependency versions.
[group('check')]
check-minimal-versions:
  cargo +stable minimal-versions check --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde
  cargo +nightly minimal-versions check --all-features

# Runs `cargo check` on a target that has no `std` library.
[group('check')]
check-nostd:
  cargo check --target thumbv7em-none-eabihf --no-default-features -F allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,alloc,serde

# Asserts that api that shouldn't panic, doesn't.
[group('check')]
check-fallibility:
  cd crates/test-fallibility && nu assert-no-panics.nu

# Runs all `test-*`.
[group('test')]
test:
  just test-stable
  just test-nightly
  just test-nightly --miri

# Runs tests on the stable toolchain.
[group('test')]
test-stable:
  cargo +stable test --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde
  cargo +stable run --example limit_memory_usage
  cargo +stable run --example stack_or_static_memory
  cargo +stable run --example thread_local
  cargo +stable test --test trybuild -- --ignored
  cargo +stable test --no-default-features --test trybuild_unavailable_panicking_macros -F alloc

# Runs tests on the nightly toolchain, optionally with miri.
[group('test'), arg("miri", long="miri", value="miri")]
test-nightly miri="": 
  cargo +nightly {{miri}} test --all-features
  cargo +nightly {{miri}} run --example limit_memory_usage
  cargo +nightly {{miri}} run --example stack_or_static_memory
  cargo +nightly {{miri}} run --example thread_local
  cd crates/tests-from-std && cargo +nightly {{miri}} test
  cd crates/test-hashbrown && cargo +nightly {{miri}} test
  cd crates/test-hashbrown && cargo +nightly {{miri}} test --all-features
  cd crates/fuzzing-support && cargo +nightly {{miri}} test

# Update the expected compile errors of `trybuild` tests.
[group('test')] 
trybuild-overwrite:
  TRYBUILD=overwrite cargo +stable test --test trybuild -- --ignored
  TRYBUILD=overwrite cargo +stable test --no-default-features --test trybuild_unavailable_panicking_macros -F alloc

# Fuzz for `seconds`.
[group('fuzz')] 
fuzz seconds:
  just fuzz-target {{seconds}} alloc_static_layout
  just fuzz-target {{seconds}} allocator_api
  just fuzz-target {{seconds}} bump_down
  just fuzz-target {{seconds}} bump_prepare_down
  just fuzz-target {{seconds}} bump_prepare_up
  just fuzz-target {{seconds}} bump_up
  just fuzz-target {{seconds}} bump_vec
  just fuzz-target {{seconds}} chunk_size
  just fuzz-target {{seconds}} slice_split_off
  just fuzz-target {{seconds}} vec_split_off

# Fuzz the chosen target for `seconds`.
[group('fuzz')] 
fuzz-target seconds target:
  cargo +nightly fuzz run {{target}} -- -max_total_time={{seconds}}

# Sync documentation using `cargo insert-docs` and run `doc-rustdoc`.
[group('doc')] 
doc *args:
  cargo +nightly fmt
  cargo insert-docs --all-features --allow-dirty
  @ just doc-rustdoc {{args}}

# Run `rustdoc` like on `docs.rs`.
[group('doc')] 
doc-rustdoc *args:
  cargo +nightly rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition -Dwarnings

# Run `rustdoc` like on `docs.rs`, but with private items.
[group('doc')] 
doc-rustdoc-private *args:
  cargo +nightly rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition -Dwarnings --document-private-items