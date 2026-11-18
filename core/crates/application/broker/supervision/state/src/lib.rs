//! Broker launch descriptions and persisted supervision state, without CLI parsing.
mod constants;
mod launch;
mod readiness;
mod record;
mod request;
mod scavenge;
pub use launch::{ChildLaunch, CredentialSource};
pub use readiness::BrokerReadiness;
pub use record::{SupervisionPhase, SupervisionRecord, launch_id};
pub use request::{SupervisionPaths, io_kind};

#[cfg(test)]
#[path = "../test_support/constants.rs"]
mod fixture_constants;
#[cfg(test)]
#[path = "../test_support/launch.rs"]
mod launch_fixture;
#[cfg(test)]
use yolo_application_test_fixtures::executables as test_executables;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
