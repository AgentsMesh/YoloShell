use anyhow::{Result, ensure};
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use yolo_distribution_bundle::{
    Architecture, ArtifactDescriptor, ArtifactRole, BIN_DIRECTORY, BundleArtifacts,
    BundleDescriptor, MANIFEST_FILE_NAME, Platform, Target,
};
use yolo_release_contracts::*;
use yolo_release_evidence_model::LinuxRuntimeEvidence;
use yolo_release_foundation::digest::{copy_opened_bounded, opened_file_sha256};
use yolo_release_gate_system::{GateSnapshot, LinuxGateProfile};
use yolo_release_linux_elf as linux_elf;
#[path = "linux_bundle_version.rs"]
mod version;
pub(crate) use version::{VERSION_SIZE_ERROR, VERSION_TYPE_ERROR};

const ROOT_BUILD: &str =
    "exports_files([\"manifest.json\"], visibility = [\"//visibility:public\"])\n";
const ARCH_BUILD: &str =
    "exports_files([\"manifest.json\", \"bin/yolo\"], visibility = [\"//visibility:public\"])\n";
const MODULE: &str = "module(name = \"yolo_linux_artifacts\")\n";

#[derive(Serialize)]
struct RuntimeManifest<'a> {
    schema_version: u16,
    source_revision: &'a str,
    runtimes: &'a [LinuxRuntimeEvidence],
}

pub fn prepare(snapshot: &GateSnapshot) -> Result<()> {
    write_file(
        snapshot.linux_artifacts().display_path(),
        LINUX_ARTIFACT_BUILD_FILE,
        ROOT_BUILD.as_bytes(),
        PRIVATE_FILE_MODE,
    )?;
    write_file(
        snapshot.linux_artifacts().display_path(),
        LINUX_ARTIFACT_MODULE_FILE,
        MODULE.as_bytes(),
        PRIVATE_FILE_MODE,
    )?;
    Ok(())
}

pub(crate) fn install(
    snapshot: &GateSnapshot,
    profile: LinuxGateProfile,
    output: &Path,
    revision: &str,
) -> Result<LinuxRuntimeEvidence> {
    yolo_release_foundation::validation::validate_revision(revision)?;
    let verified = ArtifactRole::ALL
        .into_iter()
        .map(|role| {
            version::verify(output, role)?;
            Ok((
                role,
                linux_elf::verify(&output.join(role.executable_name()), profile.elf_machine)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let destination = architecture_directory(snapshot, profile)?;
    write_file(
        &destination,
        LINUX_ARTIFACT_BUILD_FILE,
        ARCH_BUILD.as_bytes(),
        PRIVATE_FILE_MODE,
    )?;
    fs::create_dir(destination.join(BIN_DIRECTORY))?;
    let artifacts = verified
        .into_iter()
        .map(|(role, verified)| install_artifact(&destination, role, verified))
        .collect::<Result<Vec<_>>>()?;
    let [runtime]: [ArtifactDescriptor; ArtifactRole::ALL.len()] = artifacts
        .try_into()
        .map_err(|_| anyhow::anyhow!("incomplete Linux bundle"))?;
    let architecture = match profile.elf_machine {
        ELF_MACHINE_X86_64 => Architecture::X86_64,
        ELF_MACHINE_AARCH64 => Architecture::Aarch64,
        _ => anyhow::bail!("unsupported Linux bundle architecture"),
    };
    let descriptor = BundleDescriptor::new(
        revision,
        LINUX_RUNTIME_VERSION,
        Target {
            platform: Platform::Linux,
            architecture,
        },
        BundleArtifacts { runtime },
    )?;
    write_file(
        &destination,
        MANIFEST_FILE_NAME,
        &serde_json::to_vec_pretty(&descriptor)?,
        PRIVATE_FILE_MODE,
    )?;
    Ok(LinuxRuntimeEvidence {
        image: profile.image.into(),
        bazel_sha256: profile.bazel_sha256.into(),
        bundle_root: profile.bundle_root.into(),
        descriptor,
    })
}

fn install_artifact(
    destination: &Path,
    role: ArtifactRole,
    mut verified: linux_elf::VerifiedLinuxElf,
) -> Result<ArtifactDescriptor> {
    let path = destination.join(role.relative_path());
    let mut artifact = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)?;
    verified.file.seek(SeekFrom::Start(0))?;
    copy_opened_bounded(
        &verified.file,
        &mut artifact,
        LINUX_RUNTIME_MAX_BYTES,
        "Linux runtime",
    )?;
    artifact.sync_all()?;
    fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_EXECUTABLE_MODE))?;
    ensure!(
        opened_file_sha256(&artifact)? == verified.sha256,
        "copied Linux runtime changed"
    );
    ArtifactDescriptor::new(verified.sha256, artifact.metadata()?.len()).map_err(Into::into)
}

pub fn seal(
    snapshot: &mut GateSnapshot,
    revision: &str,
    runtimes: &[LinuxRuntimeEvidence],
) -> Result<()> {
    yolo_release_gate_system::validate_linux_runtimes(runtimes)?;
    ensure!(
        runtimes
            .iter()
            .all(|runtime| runtime.descriptor.source_revision() == revision),
        "Linux bundle source revision mismatch"
    );
    let manifest = RuntimeManifest {
        schema_version: LINUX_RUNTIME_MANIFEST_SCHEMA,
        source_revision: revision,
        runtimes,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    write_file(
        snapshot.linux_artifacts().display_path(),
        LINUX_RUNTIME_MANIFEST_NAME,
        &bytes,
        PRIVATE_FILE_MODE,
    )?;
    snapshot.seal_linux_artifacts()
}

fn architecture_directory(snapshot: &GateSnapshot, profile: LinuxGateProfile) -> Result<PathBuf> {
    let path = snapshot
        .linux_artifacts()
        .display_path()
        .join(profile.docker_architecture);
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn write_file(directory: &Path, name: &str, bytes: &[u8], mode: u32) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(directory.join(name))?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
