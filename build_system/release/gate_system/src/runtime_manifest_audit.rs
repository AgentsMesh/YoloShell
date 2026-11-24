use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::Path;

use yolo_release_contracts::LINUX_RUNTIME_MANIFEST_SCHEMA;
use yolo_release_evidence_model::LinuxRuntimeEvidence;
use yolo_release_foundation::file_seal::FileSeal;
use yolo_release_foundation::validation::{validate_revision, validate_sha256};

pub(crate) const MAX_RUNTIME_MANIFEST_BYTES: u64 = 64 * 1024;
const EXPECTED_RUNTIME_COUNT: usize = 2;
const CHANGED_ERROR: &str = "Linux runtime manifest changed during strict audit";
const OPEN_CONTEXT: &str = "open Linux runtime manifest without following a symlink";
const MANIFEST_START_OFFSET: u64 = 0;
pub const AUDIT_MANIFEST_OPERATION: &str = "audit-manifest";
pub const AUDIT_BUNDLE_OPERATION: &str = "audit-bundle";
pub const MANIFEST_ARGUMENT: &str = "manifest";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictManifest {
    schema_version: u16,
    source_revision: String,
    runtimes: Vec<LinuxRuntimeEvidence>,
}

pub(crate) fn audit_opened_manifest(
    file: &File,
    expected_revision: &str,
    expected_runtimes: &[LinuxRuntimeEvidence],
) -> Result<()> {
    let bytes = read_opened(file)?;
    let manifest = parse_and_validate(&bytes)?;
    validate_revision(expected_revision)?;
    validate_runtimes(expected_runtimes)?;
    ensure!(
        manifest.source_revision == expected_revision,
        "Linux runtime manifest revision differs from frozen source"
    );
    ensure!(
        manifest.runtimes == expected_runtimes,
        "Linux runtime manifest differs from sealed runtime evidence"
    );
    Ok(())
}

pub fn audit_runtime_manifest_path(path: &Path) -> Result<()> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .context(OPEN_CONTEXT)?;
    parse_and_validate(&read_opened(&file)?).map(|_| ())
}

pub fn audit_bundle_manifest_path(path: &Path) -> Result<()> {
    use yolo_distribution_bundle::{Architecture, ArtifactRole, BundleDescriptor, Platform};
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .context(OPEN_CONTEXT)?;
    let descriptor: BundleDescriptor = serde_json::from_slice(&read_opened(&file)?)?;
    ensure!(
        descriptor.target().platform == Platform::Linux,
        "release resource bundle must target Linux"
    );
    let machine = match descriptor.target().architecture {
        Architecture::X86_64 => yolo_release_contracts::ELF_MACHINE_X86_64,
        Architecture::Aarch64 => yolo_release_contracts::ELF_MACHINE_AARCH64,
    };
    let root = path.parent().context("bundle manifest has no parent")?;
    for role in ArtifactRole::ALL {
        let verified = yolo_release_linux_elf::verify(&root.join(role.relative_path()), machine)?;
        let expected = descriptor.artifact(role);
        ensure!(
            verified.sha256 == expected.sha256()
                && verified.file.metadata()?.len() == expected.size_bytes(),
            "packaged runtime bundle bytes differ from descriptor"
        );
    }
    Ok(())
}

fn read_opened(file: &File) -> Result<Vec<u8>> {
    let seal = FileSeal::capture(file)?;
    ensure!(
        seal.length() <= MAX_RUNTIME_MANIFEST_BYTES,
        "Linux runtime manifest exceeds byte budget"
    );
    let length = usize::try_from(seal.length()).context("manifest length conversion")?;
    let mut bytes = vec![0; length];
    file.read_exact_at(&mut bytes, MANIFEST_START_OFFSET)
        .context("read sealed Linux runtime manifest")?;
    seal.verify(file, CHANGED_ERROR)?;
    Ok(bytes)
}

#[cfg(test)]
pub(crate) fn audit_manifest_bytes(
    bytes: &[u8],
    expected_revision: &str,
    expected_runtimes: &[LinuxRuntimeEvidence],
) -> Result<()> {
    let manifest = parse_and_validate(bytes)?;
    validate_revision(expected_revision)?;
    validate_runtimes(expected_runtimes)?;
    ensure!(
        manifest.source_revision == expected_revision,
        "Linux runtime manifest revision differs from frozen source"
    );
    ensure!(
        manifest.runtimes == expected_runtimes,
        "Linux runtime manifest differs from sealed runtime evidence"
    );
    Ok(())
}

fn parse_and_validate(bytes: &[u8]) -> Result<StrictManifest> {
    ensure!(
        bytes.len() as u64 <= MAX_RUNTIME_MANIFEST_BYTES,
        "Linux runtime manifest exceeds byte budget"
    );
    let manifest: StrictManifest =
        serde_json::from_slice(bytes).context("strictly parse Linux runtime manifest")?;
    ensure!(
        manifest.schema_version == LINUX_RUNTIME_MANIFEST_SCHEMA,
        "Linux runtime manifest schema mismatch"
    );
    validate_revision(&manifest.source_revision)?;
    validate_runtimes(&manifest.runtimes)?;
    ensure!(
        manifest
            .runtimes
            .iter()
            .all(|runtime| runtime.descriptor.source_revision() == manifest.source_revision),
        "bundle source revision differs from manifest"
    );
    Ok(manifest)
}

fn validate_runtimes(runtimes: &[LinuxRuntimeEvidence]) -> Result<()> {
    ensure!(
        runtimes.len() == EXPECTED_RUNTIME_COUNT,
        "Linux runtime manifest must contain exactly two runtimes"
    );
    crate::validation_linux::validate(runtimes)?;
    let mut architectures = HashSet::with_capacity(EXPECTED_RUNTIME_COUNT);
    let mut bundle_paths = HashSet::with_capacity(EXPECTED_RUNTIME_COUNT);
    for runtime in runtimes {
        ensure!(
            architectures.insert(runtime.descriptor.target().architecture.as_str()),
            "Linux runtime architecture is duplicated"
        );
        ensure!(
            bundle_paths.insert(runtime.bundle_root.as_str()),
            "Linux runtime bundle path is duplicated"
        );
        validate_sha256(&runtime.bazel_sha256)?;
        validate_sha256(runtime.descriptor.build_id())?;
    }
    Ok(())
}
