# This was written for nushell version 0.91.0
set shell := ["nu", "-c"]

export RUST_BACKTRACE := "1"
export MIRIFLAGS := "-Zmiri-strict-provenance"

default *args:
  cargo fmt --all
  cargo clippy --all --tests

all:
  just fmt
  just clippy
  just doc
  just check-nostd
  just check-msrv
  just check-nooom
  cargo test --all-features
  cargo miri test --all-features
  just test-fallibility
  cargo +stable semver-checks

all-fast:
  just fmt
  just clippy
  just doc-fast
  just check-nostd
  just check-msrv
  just check-nooom
  just test-fallibility

fmt:
  cargo fmt --all
  cd crates/fuzzing-support; cargo fmt --all
  cd crates/inspect-asm; cargo fmt --all
  cd crates/test-fallibility; cargo fmt --all
  cd fuzz; cargo fmt --all

clippy:
  cargo clippy --all --tests
  cargo clippy --all --tests --no-default-features
  cargo clippy --all --tests --no-default-features --features alloc
  cd crates/fuzzing-support; cargo clippy --all --tests
  cd crates/inspect-asm; cargo clippy --all --tests
  cd crates/test-fallibility; cargo clippy --all --tests
  cd fuzz; cargo clippy --all

spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}"

doc *args:
  cargo test --package bump-scope --lib --all-features -- insert_feature_docs --exact --ignored
  cargo fmt
  @ just doc-fast {{args}}
  # TODO(blocked): stop stripping links when <https://github.com/orium/cargo-rdme/pull/236> is merged
  cargo rdme --force --intralinks-strip-links

doc-fast *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition

doc-priv *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition --document-private-items

check-nostd:
  cd crates/test-fallibility; cargo check

check-msrv:
  cargo ('+' + (open Cargo.toml).package.rust-version) check

# allocator-api2 doesn't work with no_global_oom_handling, we need nightly
check-nooom:
  RUSTFLAGS="--cfg no_global_oom_handling" cargo check --features nightly-allocator-api

test-fallibility:
  @ just crates/test-fallibility/test

inspect-asm *args:
  just crates/inspect-asm/inspect {{args}}
