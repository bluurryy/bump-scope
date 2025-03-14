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
  cargo test --all-features
  cargo miri test --all-features

check: 
  just check-fmt
  just check-clippy
  just check-nostd
  just check-msrv
  just check-fallibility

check-fmt:
  cargo fmt --check
  cd crates/fuzzing-support; cargo fmt --check
  cd crates/test-fallibility; cargo fmt --check
  cd fuzz; cargo fmt --check

check-clippy:
  cargo clippy --tests --all-features
  cargo clippy --no-default-features
  cd crates/fuzzing-support; cargo clippy --tests
  cd crates/test-fallibility; cargo clippy --tests
  cd fuzz; cargo clippy

check-nostd:
  cd crates/test-fallibility; cargo check

check-msrv:
  cargo ('+' + (open Cargo.toml).package.rust-version) check --no-default-features
  cargo ('+' + (open Cargo.toml).package.rust-version) check --features alloc,panic-on-alloc,serde,zerocopy
  cargo ('+' + (open Cargo.toml).package.rust-version) check --features std,panic-on-alloc,serde,zerocopy

check-fallibility:
  @ just crates/test-fallibility/test

fmt:
  cargo fmt
  cd crates/fuzzing-support; cargo fmt
  cd crates/test-fallibility; cargo fmt
  cd fuzz; cargo fmt

spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}" --exclude src/tests/from_std

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