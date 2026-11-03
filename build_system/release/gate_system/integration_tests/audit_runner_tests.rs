use super::runtime_manifest_fixture::valid_bytes as valid_manifest_bytes;
use super::support::{ARCHIVE_BYTES, OTHER_ARCHIVE_BYTES, REVISION, error_text, linux_runtimes};
use super::test_constants::{
    FIFO_AUDIT_SCRIPT, OVERSIZED_AUDIT_SCRIPT, PASSING_AUDIT_SCRIPT, PINNED_POLICY_BYTES,
    REPEATED_ARCHIVE_AUDIT_SCRIPT, REPEATED_MANIFEST_AUDIT_SCRIPT, SWAPPING_AUDIT_SCRIPT,
    TRANSIENT_POLICY_SWAP_AUDIT_SCRIPT,
};
use crate::audit_runner::audit;
use crate::snapshot::GateSnapshot;
use std::fs;
use yolo_release_archive_audit_test_support::valid_archive;
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{
    ARCHIVE_NAME, ARCHIVE_POLICY_RELATIVE_PATH, AUDIT_SCRIPT_RELATIVE_PATH,
    BUNDLE_DEPENDENCY_POLICY_RELATIVE_PATH, BUNDLE_POLICY_RELATIVE_PATH, ICON_RELATIVE_PATH,
};

#[test]
fn fixed_auditor_reads_pinned_descriptor_after_archive_path_swap() {
    let root = tempfile::tempdir().unwrap();
    let artifact_root = root.path().join("artifact");
    fs::create_dir(&artifact_root).unwrap();
    let artifact = PinnedArtifact::fixture(&artifact_root, &valid_archive().bytes).unwrap();
    fs::write(artifact_root.join(ARCHIVE_NAME), OTHER_ARCHIVE_BYTES).unwrap();
    let snapshot = audit_snapshot(PASSING_AUDIT_SCRIPT);
    let evidence = audit(&snapshot, &artifact, REVISION, &linux_runtimes()).unwrap();
    assert_eq!(evidence.source_revision, REVISION);
    assert_eq!(evidence.artifact_sha256, artifact.sha256().unwrap());
}

#[test]
fn fixed_auditor_receives_independent_archive_readers() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    audit(
        &audit_snapshot(REPEATED_ARCHIVE_AUDIT_SCRIPT),
        &artifact,
        REVISION,
        &linux_runtimes(),
    )
    .unwrap();
}

#[test]
fn fixed_auditor_receives_independent_manifest_readers() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    audit(
        &audit_snapshot(REPEATED_MANIFEST_AUDIT_SCRIPT),
        &artifact,
        REVISION,
        &linux_runtimes(),
    )
    .unwrap();
}

#[test]
fn oversized_auditor_digest_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    let snapshot = audit_snapshot(OVERSIZED_AUDIT_SCRIPT);
    let error = error_text(audit(&snapshot, &artifact, REVISION, &linux_runtimes()));
    assert!(error.contains("too large"), "{error}");
}

#[test]
fn fifo_auditor_result_is_rejected_without_blocking() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    let snapshot = audit_snapshot(FIFO_AUDIT_SCRIPT);
    let error = error_text(audit(&snapshot, &artifact, REVISION, &linux_runtimes()));
    assert!(error.contains("must be a file"), "{error}");
}

#[test]
fn auditor_source_namespace_swap_is_detected_after_spawn() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    let snapshot = audit_snapshot(SWAPPING_AUDIT_SCRIPT);
    let source = snapshot.source().display_path().to_owned();
    let moved = source.with_extension("moved");
    let error = error_text(audit(&snapshot, &artifact, REVISION, &linux_runtimes()));
    assert!(error.contains("identity changed"), "{error}");
    fs::remove_dir(&source).unwrap();
    fs::rename(&moved, &source).unwrap();
}

#[test]
fn transient_source_swap_is_rejected_after_pinned_policy_is_consumed() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    let error = error_text(audit(
        &audit_snapshot(TRANSIENT_POLICY_SWAP_AUDIT_SCRIPT),
        &artifact,
        REVISION,
        &linux_runtimes(),
    ));
    assert!(error.contains("release source manifest changed"), "{error}");
}

#[test]
fn malformed_zip_is_rejected_before_fixed_shell_auditor_runs() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), ARCHIVE_BYTES).unwrap();
    let error = error_text(audit(
        &audit_snapshot(PASSING_AUDIT_SCRIPT),
        &artifact,
        REVISION,
        &linux_runtimes(),
    ));
    assert!(error.contains("ZIP"), "{error}");
}

#[test]
fn malformed_manifest_is_rejected_before_fixed_shell_auditor_runs() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    let snapshot = audit_snapshot(PASSING_AUDIT_SCRIPT);
    fs::write(
        snapshot
            .linux_artifacts
            .display_path()
            .join(yolo_release_contracts::LINUX_RUNTIME_MANIFEST_NAME),
        b"{}",
    )
    .unwrap();
    let error = error_text(audit(&snapshot, &artifact, REVISION, &linux_runtimes()));
    assert!(error.contains("manifest"), "{error}");
}

#[test]
fn manifest_is_bound_to_independent_runtime_evidence() {
    let root = tempfile::tempdir().unwrap();
    let artifact = PinnedArtifact::fixture(root.path(), &valid_archive().bytes).unwrap();
    let snapshot = audit_snapshot(PASSING_AUDIT_SCRIPT);
    let mut expected = linux_runtimes();
    expected.reverse();
    let error = error_text(audit(&snapshot, &artifact, REVISION, &expected));
    assert!(error.contains("runtime"), "{error}");
}

fn audit_snapshot(script: &[u8]) -> GateSnapshot {
    let snapshot = GateSnapshot::fixture_with_source(&[
        (AUDIT_SCRIPT_RELATIVE_PATH, script),
        (BUNDLE_POLICY_RELATIVE_PATH, PINNED_POLICY_BYTES),
        (ARCHIVE_POLICY_RELATIVE_PATH, PINNED_POLICY_BYTES),
        (BUNDLE_DEPENDENCY_POLICY_RELATIVE_PATH, PINNED_POLICY_BYTES),
        (ICON_RELATIVE_PATH, PINNED_POLICY_BYTES),
    ])
    .unwrap();
    fs::write(
        snapshot
            .linux_artifacts
            .display_path()
            .join(yolo_release_contracts::LINUX_RUNTIME_MANIFEST_NAME),
        valid_manifest_bytes(),
    )
    .unwrap();
    snapshot
}
