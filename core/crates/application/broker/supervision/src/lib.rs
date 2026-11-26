mod credentials;
mod server;
mod server_command;
mod start_detached;
mod supervision_ack;
mod supervision_activation;
mod supervision_frame;
mod supervisor;
mod supervisor_child;
mod supervisor_cleanup;
mod supervisor_custody;
mod supervisor_lifecycle;
mod supervisor_process;

pub use server::{prepare_server, serve};
pub use start_detached::DetachedSpawner;
pub use supervisor::run_supervisor;
pub use supervisor_child::run_supervised;
pub use yolo_broker_child_custody::{PendingBroker, PendingObservation};
pub use yolo_broker_supervision_state::{ChildLaunch, CredentialSource};

#[cfg(feature = "yolo-test-support")]
pub use server::{prepare_server_with_factory, serve_with_factory};

#[cfg(test)]
use yolo_application_test_fixtures::executables as test_executables;
#[cfg(test)]
#[path = "../test_support/constants.rs"]
mod fixture_constants;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/detached.rs"]
mod detached_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/private_input_deadline.rs"]
mod private_input_deadline_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/preparation.rs"]
mod preparation_tests;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/spawn_deadline.rs"]
mod spawn_deadline_tests;
