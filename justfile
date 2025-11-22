export RUST_BACKTRACE := "1"
export MIRIFLAGS := "-Zmiri-strict-provenance"

default:
  @just --list

pre-release:
  just spellcheck
  just doc
  just check
  just test
  cargo +stable semver-checks

# installs all tools used to run `pre-release`
setup:
  cargo binstall cargo-insert-docs@1.0.0 --locked
  cargo binstall cargo-semver-checks@0.44.0 --locked
  npm install -g cspell

check: 
  just assert-fuzz-modules-synced
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
  cargo +nightly fmt --check
  cd crates/fuzzing-support && cargo +nightly fmt --check
  cd crates/test-fallibility && cargo +nightly fmt --check
  cd crates/callgrind-benches && cargo +nightly fmt --check
  cd crates/criterion-benches && cargo +nightly fmt --check
  cd fuzz; cargo +nightly fmt --check

check-msrv:
  # msrv might print warnings that stable doesnt, we dont care
  cargo +1.85.1 check --no-default-features
  cargo +1.85.1 check --features serde,zerocopy-08,allocator-api2-02

check-clippy:
  cargo +stable clippy --tests --no-default-features -- -Dwarnings
  cargo +stable clippy --tests --features serde,zerocopy-08,allocator-api2-02,allocator-api2-03 -- -Dwarnings

  cargo +nightly clippy --tests --no-default-features -- -Dwarnings
  cargo +nightly clippy --tests --features serde,zerocopy-08,allocator-api2-02,allocator-api2-03 -- -Dwarnings
  cargo +nightly clippy --tests --all-features -- -Dwarnings

  cd crates/callgrind-benches && cargo +nightly clippy --tests --benches --workspace -- -Dwarnings
  cd crates/criterion-benches && cargo +nightly clippy --tests --benches --workspace -- -Dwarnings
  cd crates/fuzzing-support && cargo +nightly clippy --tests -- -Dwarnings
  cd crates/test-fallibility && cargo +nightly clippy --tests -- -Dwarnings
  cd crates/test-hashbrown && cargo +nightly clippy --tests -- -Dwarnings
  cd crates/tests-from-std && cargo +nightly clippy --tests -- -Dwarnings
  cd fuzz && cargo +nightly clippy -- -Dwarnings

check-nostd:
  cd crates/test-fallibility && cargo check

check-fallibility:
  cd crates/test-fallibility && nu assert-no-panics.nu

check-mustnt_compile:
  cargo +stable test --test compile_fail -- --ignored
  
check-unavailable_panicking_macros:
  cargo +stable test --no-default-features --test unavailable_panicking_macros -F alloc

sync-fuzz:
  cp -u src/bumping.rs crates/fuzzing-support/src/from_bump_scope/bumping.rs
  cp -u src/chunk_size/chunk_size_config.rs crates/fuzzing-support/src/from_bump_scope/chunk_size_config.rs

assert-fuzz-modules-synced:
  just assert-files-equal src/bumping.rs crates/fuzzing-support/src/from_bump_scope/bumping.rs
  just assert-files-equal src/chunk_size/chunk_size_config.rs crates/fuzzing-support/src/from_bump_scope/chunk_size_config.rs

assert-files-equal a b:
  @ a=`cat {{a}}`; b=`cat {{b}}`; [ "$a" = "$b" ]

test:
  just test-non-miri
  just test-miri

test-non-miri: 
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
  cd crates/fuzzing-support && cargo +nightly fmt
  cd crates/test-fallibility && cargo +nightly fmt
  cd fuzz && cargo +nightly fmt

spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}" --exclude crates/tests-from-std --exclude crates/callgrind-benches/src/schema.rs

doc *args:
  cargo +nightly fmt
  cargo insert-docs --all-features --allow-dirty
  @ just doc-rustdoc {{args}}

doc-rustdoc *args:
  cargo +nightly rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition

doc-rustdoc-priv *args:
  cargo +nightly rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition --document-private-items -definition
