use super::*;
use crate::protocol::{method, to_value};
use std::{sync::mpsc, time::Instant};

#[cfg(all(test, feature = "yolo-test-support"))]
use crate::namespace_lease::NamespaceVerification;
include!("prelude.rs");
include!("shared_test_helpers.rs");
#[path = "integration/pty/owner_support.rs"]
mod owner_support;
use owner_support::*;
#[path = "integration/pty/process_discovery.rs"]
mod process_discovery;
use process_discovery::*;

include!("support_wiring.rs");
#[cfg(all(test, feature = "yolo-unit-tests"))]
include!("unit_wiring.rs");
#[cfg(all(test, feature = "yolo-integration-tests"))]
include!("integration_wiring.rs");
