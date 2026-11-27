use crate::host::tests::*;
use super::*;

include!("../../../test_support/unit/resume/resume_convergence_support.rs");
include!("resume_completion_error_convergence.rs");
include!("../binding/resume_completion_binding_convergence.rs");
include!("resume_continuation_error_convergence.rs");
include!("resume_replay_ready_convergence.rs");
include!("resume_survivor_error_convergence.rs");
include!("resume_survivor_rearm_error_convergence.rs");
include!("resume_survivor_rearm_boundary_convergence.rs");
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../../test_support/integration/resume_provider_process_scanner_test_support.rs");
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../../test_support/integration/resume_provider_process_fixture_test_support.rs");
include!("../provider/resume_provider_binding_completion.rs");
include!("../../../test_support/unit/resume/resume_provider_binding_authority_test_support.rs");
include!("../../../test_support/unit/resume/resume_provider_binding_wait_test_support.rs");
include!("../provider/resume_provider_authority_tests.rs");
include!("../provider/resume_provider_shadow_tests.rs");
include!("../provider/resume_provider_codex_exact_authority_tests.rs");
include!("../provider/resume_provider_claude_authority_tests.rs");
include!("../provider/resume_provider_commit_race_tests.rs");
include!("../../../test_support/unit/native_claude_receipt.rs");
include!("../provider/resume_provider_binding_wait_tests.rs");
include!("../provider/resume_provider_stale_authority_tests.rs");
include!("../provider/resume_provider_binding_wait_adversarial_tests.rs");
include!("../provider/resume_provider_publication_fence_tests.rs");
include!("../provider/resume_provider_completion_pending_tests.rs");
include!("../../../test_support/unit/resume/resume_provider_scan_coordinator_test_support.rs");
include!("../provider/resume_provider_scan_coordinator_adversarial_tests.rs");
include!("../provider/resume_provider_scan_expired_tests.rs");
