//! Reconciles one authorized broker generation without owning CLI parsing or child supervision.
mod pending;
mod request;
mod retire;
mod start;
mod start_authority;

use anyhow::Result;
pub use request::{ProfileAuthorityRequest, RetireRequest, StartRequest};
pub use retire::run as retire;
pub use start::{run as start, run_hub as start_hub};
pub use yolo_broker_supervision_state::CredentialSource;
use yolo_remote_broker::{BrokerClient, BrokerConfig, BrokerPaths, BrokerTransition};

#[cfg(test)]
#[path = "../test_support/shared.rs"]
mod shared_test_support;
#[cfg(test)]
use shared_test_support::fixture as fixture_constants;
#[cfg(test)]
#[path = "../test_support/pending.rs"]
mod pending_fixture;
#[cfg(test)]
use yolo_application_test_fixtures::executables as test_executables;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
