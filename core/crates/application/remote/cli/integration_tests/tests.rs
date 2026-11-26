use super::fixture_constants as fixture;
use super::*;
use crate::options::RetireOptions;
use anyhow::Result;
use support::ready_remote;
use support::{running_fixture, shutdown_running_fixture};
use yolo_broker_startup_test_support as support;
use yolo_remote_broker::{BrokerClient, BrokerConfig, BrokerPaths};

mod authority_flow_tests;
mod conversion_effects;
mod dispatch_tests;
mod management;
#[path = "process_handoff_protocol.rs"]
mod process_handoff_protocol;
#[path = "../test_support/process_handoff_server.rs"]
mod process_handoff_server;
#[path = "process_handoff_tests.rs"]
mod process_handoff_tests;
#[path = "process_isolation_tests.rs"]
mod process_isolation_tests;
#[path = "process_namespace_tests.rs"]
mod process_namespace_tests;
#[path = "../test_support/server_options.rs"]
mod server_options;
