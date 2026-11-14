mod catalog_backpressure;
#[path = "../test_support/catalog_driver.rs"]
mod catalog_driver;
mod catalog_recovery;
#[path = "../test_support/commands.rs"]
mod commands;
#[path = "../test_support/constants.rs"]
mod constants;
mod creation_deadline;
mod handshake;
#[path = "../test_support/harness.rs"]
mod harness;
mod idle_cache;
mod independent;
mod keepalive;
#[path = "../test_support/liveness.rs"]
mod liveness;
#[path = "../test_support/maintenance.rs"]
mod maintenance;
mod maintenance_admission;
mod maintenance_isolation;
mod maintenance_recovery;
mod operation_capacity;
mod operation_generation;
mod operation_isolation;
mod operation_validity;
mod platform_events;
#[path = "../test_support/operations.rs"]
mod operations;
mod ordered_input_admission;
mod ordered_input_handshake;
mod ordered_input_outcomes;
mod ordered_input_views;
#[path = "../test_support/ordered_inputs.rs"]
mod ordered_inputs;
#[path = "../test_support/peer.rs"]
mod peer;
#[path = "../test_support/platform.rs"]
mod platform;
#[path = "../test_support/queries.rs"]
mod queries;
mod query_capacity;
#[path = "../test_support/query_constants.rs"]
mod query_constants;
mod query_deadline;
mod query_fences;
mod query_isolation;
#[path = "../test_support/query_requests.rs"]
mod query_requests;
mod query_shutdown;
mod reconnect;
mod replacement;
mod request_validity;
mod request_validity_reconnect;
#[path = "../test_support/scoped_requests.rs"]
mod scoped_requests;
