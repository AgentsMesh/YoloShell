//! Admission and capability negotiation for Yolo's bundled TUI client.
//!
//! This crate has no manifest, process launcher or UI authority. The TUI is a
//! built-in descriptor; external trusted plugins use the Hub public API directly.

mod capability;
mod descriptor;
mod entrypoint;
mod negotiation;

pub use capability::{CapabilityNegotiation, DisabledFeature};
pub use descriptor::{BuiltinClientDescriptor, ClientDescriptorError};
pub use entrypoint::{Executable, ProcessEntrypoint};
pub use negotiation::negotiate;
pub use yolo_plugin_api::{ClientApiContract, ClientCapability, ClientFeatureRequirement};
