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

check: 
  just assert-fuzz-modules-synced
  just check-fmt
  just check-clippy
  just check-nostd
  just check-fallibility
  # regression test making sure hashbrown compiles
  cargo check --tests --features nightly-allocator-api 

check-fmt:
  cargo fmt --check
  cd crates/fuzzing-support && cargo fmt --check
  cd crates/test-fallibility && cargo fmt --check
  cd crates/callgrind-benches && cargo fmt --check
  cd crates/criterion-benches && cargo fmt --check
  cd fuzz; cargo fmt --check

check-clippy:
  # TODO: add "allocator-api2-03" once it got a new release that makes its "alloc" feature msrv compliant
  cargo +1.64.0 check --no-default-features
  cargo +1.64.0 check --features serde,zerocopy-08,allocator-api2-02

  cargo +stable clippy --tests --no-default-features
  cargo +stable clippy --tests --features serde,zerocopy-08,allocator-api2-02,allocator-api2-03

  cargo +nightly clippy --tests --no-default-features
  cargo +nightly clippy --tests --features serde,zerocopy-08,allocator-api2-02,allocator-api2-03
  cargo +nightly clippy --tests --all-features

  cd crates/fuzzing-support && cargo clippy --tests
  cd crates/test-fallibility && cargo clippy --tests
  cd crates/tests-from-std && cargo clippy --tests
  cd crates/callgrind-benches && cargo clippy --tests --benches --workspace
  cd crates/criterion-benches && cargo clippy --tests --benches --workspace
  cd fuzz && cargo clippy

check-nostd:
  cd crates/test-fallibility && cargo check

check-fallibility:
  cd crates/test-fallibility && nu assert-no-panics.nu

assert-fuzz-modules-synced:
  just assert-files-equal src/bumping.rs crates/fuzzing-support/src/from_bump_scope/bumping.rs
  just assert-files-equal src/chunk_size/chunk_size_config.rs crates/fuzzing-support/src/from_bump_scope/chunk_size_config.rs

assert-files-equal a b:
  @ a=`cat {{a}}`; b=`cat {{b}}`; [ "$a" = "$b" ]

test:
  just test-non-miri
  just test-miri

test-non-miri: 
  cargo test --all-features
  cd crates/tests-from-std && cargo test
  cd crates/test-hashbrown && cargo test
  cd crates/test-hashbrown && cargo test --all-features
  cd crates/fuzzing-support && cargo test

test-miri:
  cargo miri test --all-features
  cd crates/tests-from-std && cargo miri test
  cd crates/test-hashbrown && cargo miri test
  cd crates/test-hashbrown && cargo miri test --all-features
  cd crates/fuzzing-support && cargo miri test

fmt:
  cargo fmt
  cd crates/fuzzing-support && cargo fmt
  cd crates/test-fallibility && cargo fmt
  cd fuzz && cargo fmt

spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}" --exclude crates/tests-from-std --exclude crates/callgrind-benches/src/schema.rs

doc *args:
  cargo test --package bump-scope --lib --all-features -- insert_feature_docs --exact --ignored
  cargo fmt
  @ cargo rustdoc --all-features -- --cfg docsrs -Z unstable-options --output-format json
  nu insert-docs-into-readme.nu
  @ just doc-rustdoc {{args}}

doc-rustdoc *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition

doc-rustdoc-priv *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition --document-private-items