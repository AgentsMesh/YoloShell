//! Bounded directory routing, not authentication or execution authority.
//!
//! The peer session layer authenticates the immediate peer and validates origin
//! delegation before installing a grant. Untrusted wire data must not call
//! `authorize` directly. Forwarded operations still require origin-side checks.
mod catalog;
mod constants;
mod error;
mod export;
mod ingest;
mod limits;
mod model;
mod peers;
mod resource;
mod rights;
mod views;

pub use catalog::ResourceCatalog;
pub use resource::Resource;
pub type Catalog = ResourceCatalog<yolo_hub_identity::AgentAddress>;
pub type SessionCatalog = ResourceCatalog<yolo_hub_identity::TerminalAddress>;
pub type HubCatalog = ResourceCatalog<yolo_hub_identity::HubId>;
pub use error::RoutingError;
pub use limits::RoutingLimits;
pub use model::{CatalogEntry, Grant, PeerConnection, Route, RouteAdvertisement};
pub use rights::{Right, Rights};
