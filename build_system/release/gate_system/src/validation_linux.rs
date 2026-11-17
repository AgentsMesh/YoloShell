use crate::requirements::{GateKind, REQUIRED_GATES};
use anyhow::{Result, ensure};
use yolo_release_contracts::{LINUX_RUNTIME_PLATFORM, LINUX_RUNTIME_VERSION};
use yolo_release_evidence_model::LinuxRuntimeEvidence;
use yolo_release_foundation::validation::validate_sha256;

pub fn validate(runtimes: &[LinuxRuntimeEvidence]) -> Result<()> {
    let profiles = REQUIRED_GATES
        .iter()
        .filter_map(|requirement| match requirement.kind {
            GateKind::Linux(profile) => Some(profile),
            GateKind::Bazel(_) => None,
        });
    ensure!(
        runtimes.len() == profiles.clone().count(),
        "Linux runtime evidence count mismatch"
    );
    for (runtime, profile) in runtimes.iter().zip(profiles) {
        ensure!(
            runtime.descriptor.target().architecture.as_str() == profile.architecture,
            "Linux runtime architecture mismatch"
        );
        ensure!(
            runtime.descriptor.target().platform.as_str() == LINUX_RUNTIME_PLATFORM,
            "Linux runtime platform mismatch"
        );
        ensure!(
            runtime.image == profile.image,
            "Linux runtime image mismatch"
        );
        ensure!(
            runtime.bazel_sha256 == profile.bazel_sha256,
            "Linux runtime Bazel mismatch"
        );
        ensure!(
            runtime.descriptor.version() == LINUX_RUNTIME_VERSION,
            "Linux runtime version mismatch"
        );
        validate_sha256(runtime.descriptor.build_id())?;
        ensure!(
            runtime.bundle_root == profile.bundle_root,
            "Linux runtime bundle path mismatch"
        );
    }
    Ok(())
}

pub fn find<'a>(
    runtimes: &'a [LinuxRuntimeEvidence],
    requirement: GateRequirement,
) -> Option<&'a LinuxRuntimeEvidence> {
    let GateKind::Linux(profile) = requirement.kind else {
        return None;
    };
    runtimes
        .iter()
        .find(|runtime| runtime.descriptor.target().architecture.as_str() == profile.architecture)
}

use crate::requirements::GateRequirement;
