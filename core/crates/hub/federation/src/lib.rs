mod constants;
mod discovery;
mod discovery_channel;
mod dispatch;
mod forward_diagnostics;
mod hub_inventory;
mod incoming;
mod lifetime;
mod network;
mod node;
mod observation_admission;
mod observation_cache;
mod observation_mailbox;
mod observation_worker;
mod origin_stream;
mod read_reply_admission;
mod registry;
mod relay;
mod remote_source;
mod retirement;
mod run;
mod streams;

pub use constants::{CONNECT_TIMEOUT, OPERATION_TIMEOUT, POLL_INTERVAL, STOP_TIMEOUT};
pub use node::Federation;
pub use yolo_hub_federation_directory::ObservationHistory;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/aggregated_platform.rs"]
mod aggregated_platform;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/dispatch_inventory.rs"]
mod dispatch_inventory;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lifetime_bridge.rs"]
mod lifetime_bridge;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lifetime_origin.rs"]
mod lifetime_origin;
