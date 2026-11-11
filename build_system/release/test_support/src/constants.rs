pub const TEST_EXACT_ARGUMENT: &str = "--exact";
pub const TEST_SRCDIR_ENVIRONMENT: &str = "TEST_SRCDIR";
pub const TEST_WORKSPACE_ENVIRONMENT: &str = "TEST_WORKSPACE";
pub const RELEASE_EVIDENCE_DOCUMENT: &str = "docs/release-evidence.md";
pub const BLOCKING_IO_BOUNDARY_TEXT: &str =
    "do not provide a wall-clock deadline against a malicious or indefinitely blocking filesystem";
pub const TRUSTED_FILESYSTEM_BOUNDARY_TEXT: &str = "trusted, responsive filesystems";
pub const BLOCKING_FILESYSTEM_RESIDUAL_TEXT: &str =
    "The blocking-filesystem limit is an accepted containment residual";
pub const DISCOVERY_ENDPOINT_CUSTODY_TEXT: &str = "management custody retains only the discovery reader; the launcher, exact root, and descendants inherit only the writer";
pub const DISCOVERY_EXACT_ACKNOWLEDGEMENT_TEXT: &str = "The pre-exec acknowledgement binds the exact root identity and both nonzero, distinct reciprocal endpoint handles, proves that exact root holds the writer, then drops the issuer's writer while the spawn gate is still held";
pub const DISCOVERY_TRANSFERABLE_UNIVERSE_TEXT: &str = "Descriptor-backed discovery scans the complete stable same-UID inventory, including unrelated or pre-root processes that can receive the writer through `dup` or `SCM_RIGHTS`; such a transfer extends custody instead of escaping it";
pub const DISCOVERY_INDEPENDENT_CANDIDATE_TEXT: &str = "Every member proves the exact writer handle; environment evidence only discovers or classifies candidates and cannot prove membership, exclusion, or settlement";
pub const DISCOVERY_WRITER_PENDING_TEXT: &str =
    "Writer presence keeps discovery publication pending";
pub const DISCOVERY_STICKY_POISON_TEXT: &str = "Premature EOF and indeterminate release observations are sticky poison and later writer closure cannot turn either into clean release";
pub const DISCOVERY_RELEASED_PTY_PENDING_TEXT: &str =
    "Sequential negative scans remain `Pending` regardless of candidate order";
pub const DISCOVERY_GLOBAL_PTY_EXCLUSION_TEXT: &str =
    "Only a future kernel-global PTY exclusion primitive may authorize empty completion";
pub const LIVE_BOOTSTRAP_BOUNDARY_TEXT: &str =
    "The outer Bazel wrapper and producer executable are built from the live workspace";
pub const SELF_ATTESTATION_BOUNDARY_TEXT: &str =
    "cannot use that producer to prove that the producer itself was not maliciously replaced";
pub const BAZEL_RUNFILES_ROOT_DETAIL: &str = "Bazel runfiles root";
pub const BAZEL_WORKSPACE_NAME_DETAIL: &str = "Bazel workspace name";
pub const RELEASE_EVIDENCE_READ_DETAIL: &str = "release evidence document";
pub const NORMALIZED_WHITESPACE_SEPARATOR: &str = " ";
pub const ABRUPT_OUTPUT_ROOT_ENVIRONMENT: &str = "YOLO_TEST_ABRUPT_OUTPUT_ROOT";
pub const ABRUPT_OUTPUT_TEST_FILTER: &str = "abrupt_exit_before_install_never_publishes_candidate";
pub const PUBLICATION_CRASH_ROOT_ENVIRONMENT: &str = "YOLO_TEST_PUBLICATION_CRASH_ROOT";
pub const PUBLICATION_CRASH_CHECKPOINT_ENVIRONMENT: &str = "YOLO_TEST_PUBLICATION_CRASH_CHECKPOINT";
pub const PUBLICATION_CRASH_TEST_FILTER: &str = "integration_tests::output_crash_tests::publication_crash_checkpoints_distinguish_marker_durability";
pub const RENAMED_CHECKPOINT: &str = "renamed";
pub const INITIAL_SYNC_CHECKPOINT: &str = "initial_sync_complete";
pub const ENTRIES_REOPENED_CHECKPOINT: &str = "entries_reopened";
pub const CONTENT_VERIFIED_CHECKPOINT: &str = "content_verified";
pub const MODE_PROTECTED_CHECKPOINT: &str = "mode_protected";
pub const MARKER_APPLIED_CHECKPOINT: &str = "marker_applied";
pub const CANDIDATE_BARRIER_CHECKPOINT: &str = "candidate_barrier_complete";
pub const DURABILITY_BARRIERS_CHECKPOINT: &str = "durability_barriers_complete";
pub const COMMIT_MARKED_CHECKPOINT: &str = "commit_marked";
pub const FINAL_VALIDATION_CHECKPOINT: &str = "final_validation_complete";
pub const TEST_SUCCESS_EXIT_STATUS: libc::c_int = 0;
pub const OUTPUT_TEST_DESTINATION_NAME: &str = "output";
pub const OUTPUT_REBOUND_NAME: &str = "rebound-output";
pub const OUTPUT_REPLACED_BACKUP_NAME: &str = "replaced-output-entry";
pub const OUTPUT_EXTRA_NAME: &str = "unexpected-output";
pub const OUTPUT_EXTERNAL_LINK_NAME: &str = "external-output-link";
pub const OUTPUT_TRANSIENT_NAME: &str = "transient-parent-entry";
pub const INJECTED_PUBLICATION_FAILURE: &str = "injected precommit verification failure";
pub const INJECTED_INITIAL_SYNC_FAILURE: &str = "injected initial parent sync failure";
pub const INJECTED_CANDIDATE_COMMIT_SYNC_FAILURE: &str =
    "injected candidate commit durability failure";
pub const INJECTED_PARENT_COMMIT_SYNC_FAILURE: &str = "injected parent commit durability failure";
pub const INJECTED_ACKNOWLEDGEMENT_FAILURE: &str = "injected publication acknowledgement failure";
pub const TRANSIENT_PARENT_BYTES: &[u8] = b"transient parent mutation\n";
pub const CHMOD_EXECUTABLE: &str = "/bin/chmod";
pub const ACL_ADD_ARGUMENT: &str = "+a";
pub const ACL_REMOVE_ARGUMENT: &str = "-N";
pub const TEST_DENY_DELETE_ACL: &str = "everyone deny delete";
pub const UNSAFE_OUTPUT_PARENT_MODE: u32 = 0o720;
pub const TAMPERED_OUTPUT_BYTES: &[u8] = b"replaced candidate entry\n";
pub const INVALID_AUDIT_INPUT_KIND: &str = "replaceable_path";
pub const ARTIFACT_SHA256_JSON_KEY: &str = "artifact_sha256";
pub const BUNDLE_ID_JSON_KEY: &str = "bundle_id";
pub const BUNDLE_DESCRIPTOR_JSON_KEY: &str = "descriptor";
pub const BUILD_ID_JSON_KEY: &str = "build_id";
pub const SOURCE_REVISION_JSON_KEY: &str = "source_revision";
