//! Public projection of Host-validated agent bindings, never a source of process custody.
//! Only the authenticated local Host boundary may feed TerminalViews into this component.

mod constants;
mod control;
mod control_record;
mod identity;
mod observation;
mod project;
mod record;
mod store;

pub use store::AgentProjection;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
