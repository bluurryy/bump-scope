# This was written for nushell version 0.89.0
set shell := ["nu", "-c"]

default *args:
  cargo fmt --all
  cargo clippy --all --tests

all:
  just doc
  nu insert-docs-into-readme.nu
  cargo fmt --all
  cargo clippy --all --tests
  cargo clippy --all --tests --no-default-features
  cargo clippy --all --tests --no-default-features --features alloc
  cd crates/fuzzing-support; cargo fmt; cargo clippy
  just check-msrv
  just check-nooom
  cargo test
  cargo miri test
  just test-fallibility
  just inspect-asm
  
spellcheck:
  # https://www.npmjs.com/package/cspell
  cspell lint --gitignore "**/*.{rs,md,toml}"

doc *args:
  cargo rustdoc {{args}} --features nightly-coerce-unsized,nightly-exact-size-is-empty,nightly-trusted-len -- --cfg docsrs

doc-priv *args:
  cargo rustdoc {{args}} --all-features -- --cfg docsrs --cfg test --document-private-items

check-msrv:
  cargo ('+' + (open Cargo.toml).package.rust-version) check

# allocator-api2 doesn't work with no_global_oom_handling, we need nightly
check-nooom:
  RUSTFLAGS="--cfg no_global_oom_handling" cargo check --features nightly-allocator-api

test-fallibility:
  @ just crates/test-fallibility/test

inspect-asm *args:
  just crates/inspect-asm/inspect {{args}}