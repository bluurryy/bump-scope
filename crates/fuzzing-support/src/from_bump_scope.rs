//! Ideally we would write something like this:
//! ```
//! #[path = "../../../src/bumping.rs"]
//! mod bumping;
//! ```
//! But rust analyzer cannot handle it, so we copy the file verbatim.
#![allow(clippy::manual_is_multiple_of)] // msrv

#[allow(dead_code)]
pub(crate) mod bumping;

#[allow(dead_code)]
pub(crate) mod chunk_size_config;
