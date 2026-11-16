//! Linux callback lineage normalization shared by provider hook ingress.
#[cfg(target_os = "linux")]
mod normalize;
#[cfg(target_os = "linux")]
pub use normalize::normalize;
