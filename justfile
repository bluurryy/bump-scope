# This was written for nushell version 0.91.0
set shell := ["nu", "-c"]

export RUST_BACKTRACE := "1"
export MIRIFLAGS := "-Zmiri-strict-provenance"

default:
  @just --list

pre-release:
  # TODO: fix issues
  # just spellcheck
  just doc
  just check
  cargo test --all-features
  cargo miri test --all-features
  cargo +stable semver-checks

check: 
  just check-fmt
  just check-clippy
  just check-nostd
  just check-msrv
  just check-nooom
  just check-fallibility

check-fmt:
  cargo fmt --check
  cd crates/fuzzing-support; cargo fmt --check
  cd crates/inspect-asm; cargo fmt --check
  cd crates/test-fallibility; cargo fmt --check
  cd fuzz; cargo fmt --check

check-clippy:
  cargo clippy --tests
  cargo clippy --tests --no-default-features
  cargo clippy --tests --no-default-features --features alloc
  cd crates/fuzzing-support; cargo clippy --tests
  cd crates/inspect-asm; cargo clippy --tests
  cd crates/test-fallibility; cargo clippy --tests
  cd fuzz; cargo clippy

check-nostd:
  cd crates/test-fallibility; cargo check

check-msrv:
  cargo ('+' + (open Cargo.toml).package.rust-version) check

# allocator-api2 doesn't work with no_global_oom_handling, we need nightly
check-nooom:
  RUSTFLAGS="--cfg no_global_oom_handling" cargo check --features nightly-allocator-api

check-fallibility:
  @ just crates/test-fallibility/test

fmt:
  cargo fmt
  cd crates/fuzzing-support; cargo fmt
  cd crates/inspect-asm; cargo fmt
  cd crates/test-fallibility; cargo fmt
  cd fuzz; cargo fmt

spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}"

doc *args:
  cargo test --package bump-scope --lib --all-features -- insert_feature_docs --exact --ignored
  cargo fmt
  @ just doc-rustdoc {{args}}
  @# https://github.com/orium/cargo-rdme
  @# TODO(blocked): stop stripping links when <https://github.com/orium/cargo-rdme/pull/236> is merged
  cargo rdme --force --intralinks-strip-links

doc-rustdoc *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition

doc-rustdoc-priv *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs -Z unstable-options --generate-link-to-definition --document-private-items

inspect-asm *args:
  just crates/inspect-asm/inspect-asm {{args}}
