extern crate self as yolo_hub_federation_directory;

#[path = "../src/book.rs"]
mod book;
#[path = "../src/constants.rs"]
mod constants;
#[path = "../src/history.rs"]
mod observation_history;
#[path = "../src/routes.rs"]
mod routes;
#[path = "../src/views.rs"]
mod views;
pub use book::OriginBook;
pub use observation_history::ObservationHistory;

mod directory;
mod export_rights;
mod export_routes;
#[path = "../test_support/exports.rs"]
mod exports;
mod history;
mod history_conflicts;
mod history_constructor;
mod history_lifecycle;
mod history_merge;
#[path = "../test_support/history.rs"]
mod history_support;
mod history_validation;
mod hubs;
mod install_bounds;
mod install_capacity;
mod install_versions;
#[path = "../test_support/inventory.rs"]
mod inventory;
mod live_observations;
mod state_edges;
use yolo_hub_peer_protocol_test_support::{self as fixtures, support};
