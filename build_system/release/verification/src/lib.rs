pub mod acceptance_receipt_candidate;
pub mod output;
mod output_files;
mod output_publish;
mod output_publish_commit;
mod output_stage;
mod output_stage_commit;
pub mod validation;
mod validation_audit;
mod validation_gate;
pub mod verify;
pub mod verify_acceptance;

pub use verify::{VerifyRequest, verify};
pub use verify_acceptance::{VerifyAcceptanceRequest, verify_acceptance};

#[cfg(feature = "yolo-test-support")]
#[path = "../test_support/output.rs"]
pub mod test_support;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lib.rs"]
mod integration_tests;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
