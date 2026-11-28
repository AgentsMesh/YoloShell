//! Neutral identities for upgrading one local broker connection into a Hub tunnel.
#[cfg(test)]
extern crate self as yolo_broker_hub_protocol;
mod binding;
mod constants;
mod error;
mod request;

pub use binding::{BrokerHubConnectionBinding, BrokerHubProfile};
pub use constants::{HUB_TRANSPORT_CAPABILITY, HUB_TRANSPORT_FIELD};
pub use error::BrokerHubError;
pub use request::{BrokerHubOpenRequest, BrokerHubOpened};

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod tests;
