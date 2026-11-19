//! Pins committed profile authority or an exact retirement mutation across process handoff.
mod digest;
mod document;
mod guard;
mod secure_file;
mod shape;

pub use guard::{RetirementAuthorityGuard, StartAuthorityGuard};
