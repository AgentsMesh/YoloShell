use crate::LinuxGateProfile;
use anyhow::{Result, ensure};
use std::path::PathBuf;
use yolo_release_contracts::*;
use yolo_release_evidence_model::GateExecutionEvidence;
use yolo_release_foundation::digest::bytes_sha256;

/// A unique snapshot owns two distinct engine resources, never a name prefix.
#[derive(Clone, Debug)]
pub struct ContainerCommand {
    pub name: String,
    pub owner: String,
    pub cid_file: PathBuf,
}

impl ContainerCommand {
    pub fn new(
        profile: LinuxGateProfile,
        execution: &GateExecutionEvidence,
        role: &str,
    ) -> Result<Self> {
        ensure!(
            matches!(role, CONTAINER_BUILD_ROLE | CONTAINER_CGROUP_ROLE),
            CONTAINER_CHANGED
        );
        let owner = bytes_sha256(&serde_json::to_vec(&(
            &execution.workspace_snapshot,
            &execution.isolated_tmp,
            &execution.namespace_identity,
            profile.architecture,
            profile.image,
            role,
        ))?);
        let name = format!("{CONTAINER_NAME_PREFIX}{owner}");
        let cid_file =
            PathBuf::from(&execution.isolated_tmp).join(format!("{name}{CONTAINER_CID_SUFFIX}"));
        Ok(Self {
            name,
            owner,
            cid_file,
        })
    }

    pub fn arguments(&self) -> Vec<String> {
        vec![
            CONTAINER_NAME_ARGUMENT.into(),
            self.name.clone(),
            CONTAINER_LABEL_ARGUMENT.into(),
            format!("{CONTAINER_OWNER_LABEL}={}", self.owner),
            CONTAINER_CID_ARGUMENT.into(),
            self.cid_file.display().to_string(),
        ]
    }
}
