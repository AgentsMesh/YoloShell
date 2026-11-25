use crate::audit_runner_result::{descriptor_path, ensure_absent, read};
use crate::runtime_manifest_audit::audit_opened_manifest;
use crate::snapshot::GateSnapshot;
use anyhow::{Result, ensure};
use yolo_release_archive_audit::audit_opened_archive;
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::{
    ARCHIVE_POLICY_RELATIVE_PATH, ARTIFACT_AUDIT_IDENTIFIER, ARTIFACT_AUDIT_INPUT_KIND,
    AUDIT_ARCHIVE_POLICY_DESCRIPTOR, AUDIT_ARTIFACT_AFTER_DESCRIPTOR,
    AUDIT_ARTIFACT_COPY_DESCRIPTOR, AUDIT_ARTIFACT_DESCRIPTOR, AUDIT_ARTIFACT_VERIFY_DESCRIPTOR,
    AUDIT_BUNDLE_POLICY_DESCRIPTOR, AUDIT_DEPENDENCY_POLICY_DESCRIPTOR, AUDIT_ICON_DESCRIPTOR,
    AUDIT_LINUX_MANIFEST_DESCRIPTOR, AUDIT_LINUX_MANIFEST_VERIFY_DESCRIPTOR, AUDIT_RESULT_NAME,
    AUDIT_SCRIPT_DESCRIPTOR, AUDIT_SCRIPT_RELATIVE_PATH, BASH_EXECUTABLE,
    BUNDLE_DEPENDENCY_POLICY_RELATIVE_PATH, BUNDLE_POLICY_RELATIVE_PATH, CHECKSUM_LINE_ENDING,
    ICON_RELATIVE_PATH, LINUX_RUNTIME_MANIFEST_INPUT_KIND, MACOS_MINIMUM_VERSION,
    YOLO_BUILD_VERSION, YOLO_MARKETING_VERSION,
};
use yolo_release_evidence_model::{ArtifactAuditEvidence, LinuxRuntimeEvidence};
use yolo_release_process_execution::ProcessCommand;

pub fn audit(
    snapshot: &GateSnapshot,
    artifact: &PinnedArtifact,
    source_revision: &str,
    expected_runtimes: &[LinuxRuntimeEvidence],
) -> Result<ArtifactAuditEvidence> {
    snapshot.verify_integrity()?;
    let digest_before = artifact.sha256()?;
    ensure_absent(snapshot.temporary())?;
    let binary_archive = artifact.independent_reader()?;
    audit_opened_archive(&binary_archive)?;
    let strict_manifest = snapshot.open_linux_manifest()?;
    audit_opened_manifest(&strict_manifest, source_revision, expected_runtimes)?;
    snapshot.verify_integrity()?;
    let source = snapshot.source();
    let temporary = snapshot.temporary();
    let audit_script = snapshot.open_source_file(AUDIT_SCRIPT_RELATIVE_PATH)?;
    let bundle_policy = snapshot.open_source_file(BUNDLE_POLICY_RELATIVE_PATH)?;
    let archive_policy = snapshot.open_source_file(ARCHIVE_POLICY_RELATIVE_PATH)?;
    let icon = snapshot.open_source_file(ICON_RELATIVE_PATH)?;
    let dependency_policy = snapshot.open_source_file(BUNDLE_DEPENDENCY_POLICY_RELATIVE_PATH)?;
    let linux_manifest = snapshot.open_linux_manifest()?;
    // Each /dev/fd consumer needs its own open file description and offset.
    let linux_manifest_verify = snapshot.open_linux_manifest()?;
    let artifact_before = artifact.independent_reader()?;
    let artifact_copy = artifact.independent_reader()?;
    let artifact_after = artifact.independent_reader()?;
    let artifact_verify = artifact.independent_reader()?;
    let arguments = vec![
        descriptor_path(AUDIT_SCRIPT_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_ARTIFACT_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_BUNDLE_POLICY_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_ARCHIVE_POLICY_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_ICON_DESCRIPTOR).display().to_string(),
        YOLO_MARKETING_VERSION.to_owned(),
        YOLO_BUILD_VERSION.to_owned(),
        MACOS_MINIMUM_VERSION.to_owned(),
        temporary
            .display_path()
            .join(AUDIT_RESULT_NAME)
            .display()
            .to_string(),
        descriptor_path(AUDIT_DEPENDENCY_POLICY_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_LINUX_MANIFEST_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_ARTIFACT_COPY_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_ARTIFACT_AFTER_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_ARTIFACT_VERIFY_DESCRIPTOR)
            .display()
            .to_string(),
        descriptor_path(AUDIT_LINUX_MANIFEST_VERIFY_DESCRIPTOR)
            .display()
            .to_string(),
    ];
    let output = ProcessCommand::new(
        BASH_EXECUTABLE,
        arguments,
        source,
        temporary,
        temporary,
        "fixed repository bundle auditor",
    )
    .retain_directories([artifact.resource_directory()])
    .inherit_as(audit_script, AUDIT_SCRIPT_DESCRIPTOR)
    .inherit_as(artifact_before, AUDIT_ARTIFACT_DESCRIPTOR)
    .inherit_as(bundle_policy, AUDIT_BUNDLE_POLICY_DESCRIPTOR)
    .inherit_as(archive_policy, AUDIT_ARCHIVE_POLICY_DESCRIPTOR)
    .inherit_as(icon, AUDIT_ICON_DESCRIPTOR)
    .inherit_as(dependency_policy, AUDIT_DEPENDENCY_POLICY_DESCRIPTOR)
    .inherit_as(linux_manifest, AUDIT_LINUX_MANIFEST_DESCRIPTOR)
    .inherit_as(artifact_copy, AUDIT_ARTIFACT_COPY_DESCRIPTOR)
    .inherit_as(artifact_after, AUDIT_ARTIFACT_AFTER_DESCRIPTOR)
    .inherit_as(artifact_verify, AUDIT_ARTIFACT_VERIFY_DESCRIPTOR)
    .inherit_as(
        linux_manifest_verify,
        AUDIT_LINUX_MANIFEST_VERIFY_DESCRIPTOR,
    )
    .execute()?;
    ensure!(
        output.status.success(),
        "fixed bundle audit failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    snapshot.verify_integrity()?;
    let reported = read(temporary)?;
    ensure!(
        reported == format!("{digest_before}{CHECKSUM_LINE_ENDING}"),
        "bundle auditor reported a different archive digest"
    );
    ensure!(
        artifact.sha256()? == digest_before,
        "audited artifact changed"
    );
    Ok(ArtifactAuditEvidence {
        audit: ARTIFACT_AUDIT_IDENTIFIER.to_owned(),
        source_revision: source_revision.to_owned(),
        artifact_sha256: digest_before,
        auditor_source: AUDIT_SCRIPT_RELATIVE_PATH.to_owned(),
        artifact_input: ARTIFACT_AUDIT_INPUT_KIND.to_owned(),
        linux_runtime_manifest_input: LINUX_RUNTIME_MANIFEST_INPUT_KIND.to_owned(),
    })
}
