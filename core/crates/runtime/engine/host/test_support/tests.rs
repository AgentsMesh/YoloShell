use super::creation::*;
use super::lifecycle::*;
use super::resume::*;
use super::*;
use crate::paths::{atomic_write_json, read_json};
use crate::protocol::{RequestEnvelope, method, read_frame, to_value, write_frame};
use tempfile::tempdir;
use tokio::net::UnixListener;
const HOST_BLOCKING_TEST_TIMEOUT: Duration = Duration::from_secs(10);
const RENDEZVOUS_CHANNEL_CAPACITY: usize = 0;
#[path = "unit/recovery/manifest_snapshot.rs"]
mod manifest_snapshot;
use manifest_snapshot::*;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "unit/creation/receipt_setup.rs"]
mod receipt_setup;
#[cfg(all(test, feature = "yolo-unit-tests"))]
use receipt_setup::*;
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/resume/recovery/resume_delayed_release.rs"]
mod resume_delayed_release;

include!("prelude.rs");
include!("shared_test_helpers.rs");
include!("support_wiring.rs");
include!("unit_test_wiring_a.rs");
include!("unit_test_wiring_b.rs");
include!("integration_test_wiring.rs");
