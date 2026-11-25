use crate::LinuxGateProfile;
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
use yolo_release_contracts::*;
use yolo_release_evidence_model::GateExecutionEvidence;

const READ_ONLY_SUFFIX: &str = ",readonly";

/// Trusted bind policy shared by command rendering and engine inspection.
pub struct ContainerMounts {
    binds: Vec<BindMount>,
}

pub struct BindMount {
    source: String,
    destination: &'static str,
    writable: bool,
}

impl ContainerMounts {
    pub fn new(
        profile: LinuxGateProfile,
        execution: &GateExecutionEvidence,
        bazel: &Path,
        role: &str,
    ) -> Result<Self> {
        ensure!(
            matches!(role, CONTAINER_BUILD_ROLE | CONTAINER_CGROUP_ROLE),
            CONTAINER_CHANGED
        );
        let mut binds = vec![
            BindMount::new(
                Path::new(&execution.workspace_snapshot),
                DOCKER_WORKSPACE,
                false,
                LINUX_WORKSPACE_PATH_ERROR,
            )?,
            BindMount::new(bazel, DOCKER_BAZEL, false, LINUX_BAZEL_PATH_ERROR)?,
        ];
        if role == CONTAINER_BUILD_ROLE {
            binds.push(BindMount::new(
                &output_path(profile, execution),
                DOCKER_ARTIFACT_DIRECTORY,
                true,
                LINUX_OUTPUT_PATH_ERROR,
            )?);
        }
        Ok(Self { binds })
    }

    pub fn binds(&self) -> &[BindMount] {
        &self.binds
    }

    pub fn arguments(&self) -> Vec<String> {
        self.binds
            .iter()
            .flat_map(|mount| {
                let suffix = if mount.writable { "" } else { READ_ONLY_SUFFIX };
                [
                    DOCKER_MOUNT_ARGUMENT.into(),
                    format!(
                        "type={CONTAINER_BIND_KIND},source={},target={}{suffix}",
                        mount.source, mount.destination
                    ),
                ]
            })
            .collect()
    }
}

impl BindMount {
    fn new(
        source: &Path,
        destination: &'static str,
        writable: bool,
        error: &'static str,
    ) -> Result<Self> {
        let source_text = source.to_str().ok_or_else(|| anyhow::anyhow!(error))?;
        ensure!(
            source.is_absolute() && !source_text.contains(PATH_DELIMITER),
            error
        );
        Ok(Self {
            source: source_text.into(),
            destination,
            writable,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn destination(&self) -> &str {
        self.destination
    }

    pub fn writable(&self) -> bool {
        self.writable
    }
}

pub fn output_path(profile: LinuxGateProfile, execution: &GateExecutionEvidence) -> PathBuf {
    Path::new(&execution.isolated_tmp).join(profile.docker_architecture)
}
