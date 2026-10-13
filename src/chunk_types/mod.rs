//! Chunk types behaves somewhat like a `ManuallyDrop<T>`s in the sense that they has safe to
//! use methods that assume the chunk is still live.

pub(crate) mod r#abstract;
pub(crate) mod concrete;
pub(crate) mod dummy_memory_region;
