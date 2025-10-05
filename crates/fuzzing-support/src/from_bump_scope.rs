//! Ideally we would write something like this:
//! ```
//! #[path = "../../../src/bumping.rs"]
//! mod bumping;
//! ```
//! But rust analyzer cannot handle it, so we copy the file verbatim.

pub(crate) mod bumping;
pub(crate) mod chunk_size_config;
