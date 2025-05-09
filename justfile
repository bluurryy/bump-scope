# This was written for nushell version 0.100.0
set shell := ["nu", "-c"]

export RUST_BACKTRACE := "1"
export MIRIFLAGS := "-Zmiri-strict-provenance"

default:
  @just --list

pre-release:
  just spellcheck
  just doc
  just check
  cargo +stable semver-checks
  just test

check: 
  just check-fmt
  just check-clippy
  just check-nostd
  just check-msrv
  just check-fallibility
  # regression test making sure hashbrown compiles
  cargo check --tests --features nightly-allocator-api 

check-fmt:
  cargo fmt --check
  cd crates/fuzzing-support; cargo fmt --check
  cd crates/test-fallibility; cargo fmt --check
  cd fuzz; cargo fmt --check

check-clippy:
  cargo clippy --tests --benches --all-features
  cargo clippy --no-default-features
  cd crates/fuzzing-support; cargo clippy --tests
  cd crates/test-fallibility; cargo clippy --tests
  cd fuzz; cargo clippy

check-nostd:
  cd crates/test-fallibility; cargo check

check-msrv:
  cargo ('+' + (open Cargo.toml).package.rust-version) check --no-default-features
  cargo ('+' + (open Cargo.toml).package.rust-version) check --no-default-features --features panic-on-alloc,serde,zerocopy

check-fallibility:
  @ just crates/test-fallibility/test

test:
  just test-non-miri
  just test-miri

test-non-miri: 
  cargo test --all-features
  cd crates/tests-from-std; cargo test
  cd crates/test-hashbrown; cargo test
  cd crates/test-hashbrown; cargo test --all-features

test-miri:
  cargo miri test --all-features
  cd crates/tests-from-std; cargo miri test
  cd crates/test-hashbrown; cargo miri test
  cd crates/test-hashbrown; cargo miri test --all-features

fmt:
  cargo fmt
  cd crates/fuzzing-support; cargo fmt
  cd crates/test-fallibility; cargo fmt
  cd fuzz; cargo fmt

spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}" --exclude crates/tests-from-std

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