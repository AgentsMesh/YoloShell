mod activation;
mod admission;
mod catalogue;
mod constants;
mod enrollment;
mod observations;
mod publication;
mod registry;
mod state;

pub use admission::RuntimeAdmission;
pub use catalogue::Catalogue;
pub use registry::Registry;

#[cfg(test)]
extern crate self as yolo_hub_connector_registry;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
