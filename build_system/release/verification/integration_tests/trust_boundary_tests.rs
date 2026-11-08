use super::test_constants::{
    BAZEL_RUNFILES_ROOT_DETAIL, BAZEL_WORKSPACE_NAME_DETAIL, BLOCKING_FILESYSTEM_RESIDUAL_TEXT,
    BLOCKING_IO_BOUNDARY_TEXT, DISCOVERY_ENDPOINT_CUSTODY_TEXT,
    DISCOVERY_EXACT_ACKNOWLEDGEMENT_TEXT, DISCOVERY_GLOBAL_PTY_EXCLUSION_TEXT,
    DISCOVERY_INDEPENDENT_CANDIDATE_TEXT, DISCOVERY_RELEASED_PTY_PENDING_TEXT,
    DISCOVERY_STICKY_POISON_TEXT, DISCOVERY_TRANSFERABLE_UNIVERSE_TEXT,
    DISCOVERY_WRITER_PENDING_TEXT, LIVE_BOOTSTRAP_BOUNDARY_TEXT, NORMALIZED_WHITESPACE_SEPARATOR,
    RELEASE_EVIDENCE_DOCUMENT, RELEASE_EVIDENCE_READ_DETAIL, SELF_ATTESTATION_BOUNDARY_TEXT,
    TEST_SRCDIR_ENVIRONMENT, TEST_WORKSPACE_ENVIRONMENT, TRUSTED_FILESYSTEM_BOUNDARY_TEXT,
};
use std::path::PathBuf;

#[test]
fn blocking_regular_file_io_has_no_false_elapsed_deadline_claim() {
    let document = normalized_document();
    assert!(document.contains(BLOCKING_IO_BOUNDARY_TEXT));
    assert!(document.contains(TRUSTED_FILESYSTEM_BOUNDARY_TEXT));
    assert!(document.contains(BLOCKING_FILESYSTEM_RESIDUAL_TEXT));
}

#[test]
fn descriptor_discovery_has_fail_closed_custody() {
    let document = normalized_document();
    assert!(document.contains(DISCOVERY_ENDPOINT_CUSTODY_TEXT));
    assert!(document.contains(DISCOVERY_EXACT_ACKNOWLEDGEMENT_TEXT));
    assert!(document.contains(DISCOVERY_TRANSFERABLE_UNIVERSE_TEXT));
    assert!(document.contains(DISCOVERY_INDEPENDENT_CANDIDATE_TEXT));
    assert!(document.contains(DISCOVERY_WRITER_PENDING_TEXT));
    assert!(document.contains(DISCOVERY_STICKY_POISON_TEXT));
    assert!(document.contains(DISCOVERY_RELEASED_PTY_PENDING_TEXT));
    assert!(document.contains(DISCOVERY_GLOBAL_PTY_EXCLUSION_TEXT));
}

#[test]
fn live_producer_is_explicitly_inside_the_bootstrap_trust_boundary() {
    let document = normalized_document();
    assert!(document.contains(LIVE_BOOTSTRAP_BOUNDARY_TEXT));
    assert!(document.contains(SELF_ATTESTATION_BOUNDARY_TEXT));
}

fn normalized_document() -> String {
    let source_root = std::env::var_os(TEST_SRCDIR_ENVIRONMENT).expect(BAZEL_RUNFILES_ROOT_DETAIL);
    let workspace =
        std::env::var_os(TEST_WORKSPACE_ENVIRONMENT).expect(BAZEL_WORKSPACE_NAME_DETAIL);
    std::fs::read_to_string(
        PathBuf::from(source_root)
            .join(workspace)
            .join(RELEASE_EVIDENCE_DOCUMENT),
    )
    .expect(RELEASE_EVIDENCE_READ_DETAIL)
    .split_whitespace()
    .collect::<Vec<_>>()
    .join(NORMALIZED_WHITESPACE_SEPARATOR)
}
