mod client;
mod error;

pub use client::{FileRoots, list_roots, read};
pub use error::FileClientError;
