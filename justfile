export RUST_BACKTRACE := "1"
export MIRIFLAGS := "-Zmiri-strict-provenance"

default:
  @just --list

pre-release:
  typos
  just doc
  just check
  just test
  just test-miri
  cargo +stable semver-checks

# installs all tools used to run `pre-release`
setup:
  cargo binstall typos-cli@1.40.0 --locked
  cargo binstall cargo-insert-docs@1.1.0 --locked
  cargo binstall cargo-semver-checks@0.45.0 --locked

check: 
  just check-fmt
  just check-msrv
  just check-clippy
  just check-nostd
  just check-fallibility
  just check-mustnt_compile
  just check-unavailable_panicking_macros
  # regression test making sure hashbrown compiles
  cargo +nightly check --tests --features nightly-allocator-api 

check-fmt:
  cargo +stable fmt --check
  cd crates/fuzzing-support && cargo +stable fmt --check
  cd crates/test-fallibility && cargo +stable fmt --check
  cd crates/tests-from-std && cargo +stable fmt --all -- --check
  cd crates/callgrind-benches && cargo +stable fmt --check
  cd crates/criterion-benches && cargo +stable fmt --check
  cd fuzz; cargo +stable fmt --check

check-msrv:
  # msrv might print warnings that stable doesnt, we dont care
  cargo +1.85.1 check --no-default-features
  cargo +1.85.1 check --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde

check-clippy:
  cargo +stable clippy --tests --no-default-features -- -Dwarnings
  cargo +stable clippy --tests --features allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,serde -- -Dwarnings

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

check-nostd:
  cargo check --target thumbv7em-none-eabihf --no-default-features -F allocator-api2-02,allocator-api2-03,allocator-api2-04,bytemuck,zerocopy-08,alloc,serde

check-fallibility:
  cd crates/test-fallibility && nu assert-no-panics.nu

check-mustnt_compile:
  cargo +stable test --test compile_fail -- --ignored
  
check-unavailable_panicking_macros:
  cargo +stable test --no-default-features --test unavailable_panicking_macros -F alloc

test: 
  cargo +nightly test --all-features
  cd crates/tests-from-std && cargo +nightly test
  cd crates/test-hashbrown && cargo +nightly test
  cd crates/test-hashbrown && cargo +nightly test --all-features
  cd crates/fuzzing-support && cargo +nightly test

test-miri:
  cargo +nightly miri test --all-features
  cd crates/tests-from-std && cargo +nightly miri test
  cd crates/test-hashbrown && cargo +nightly miri test
  cd crates/test-hashbrown && cargo +nightly miri test --all-features
  cd crates/fuzzing-support && cargo +nightly miri test

fmt:
  cargo +nightly fmt
  cd crates/callgrind-benches && cargo +nightly fmt
  cd crates/criterion-benches && cargo +nightly fmt
  cd crates/fuzzing-support && cargo +nightly fmt
  cd crates/test-fallibility && cargo +nightly fmt
  cd crates/test-hashbrown && cargo +nightly fmt
  cd crates/tests-from-std && cargo +nightly fmt
  cd fuzz && cargo +nightly fmt

doc *args:
  cargo +nightly fmt
  cargo insert-docs --all-features --allow-dirty
  @ just doc-rustdoc {{args}}

doc-rustdoc *args:
  cargo +nightly rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition

doc-rustdoc-priv *args:
  cargo +nightly rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition --document-private-items -definition

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

fuzz-target seconds target:
  cargo +nightly fuzz run {{target}} -- -max_total_time={{seconds}}