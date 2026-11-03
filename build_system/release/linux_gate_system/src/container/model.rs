use anyhow::{Result, ensure};
use serde::Deserialize;
use std::collections::BTreeMap;
use yolo_release_contracts::*;
use yolo_release_evidence_model::GateExecutionEvidence;
use yolo_release_gate_system::{
    LinuxGateProfile, container_command::ContainerCommand, container_mounts::ContainerMounts,
};

const INSPECTION_NAME_ROOT: &str = "/";

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Inspection {
    pub id: String,
    pub name: String,
    pub config: Configuration,
    pub mounts: Vec<Mount>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Configuration {
    pub image: String,
    pub labels: BTreeMap<String, String>,
    pub cmd: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct Mount {
    #[serde(rename = "Type")]
    pub kind: String,
    pub source: String,
    pub destination: String,
    #[serde(rename = "RW")]
    pub writable: bool,
}

pub(crate) struct Contract {
    pub command: ContainerCommand,
    pub(super) image: String,
    pub(super) mounts: Vec<Mount>,
    pub(super) script: Vec<String>,
}
impl Contract {
    pub fn new(
        profile: LinuxGateProfile,
        execution: &GateExecutionEvidence,
        bazel: &std::path::Path,
        role: &str,
        arguments: &[String],
    ) -> Result<Self> {
        let command = ContainerCommand::new(profile, execution, role)?;
        let mounts = ContainerMounts::new(profile, execution, bazel, role)?
            .binds()
            .iter()
            .map(|mount| Mount {
                kind: CONTAINER_BIND_KIND.into(),
                source: mount.source().into(),
                destination: mount.destination().into(),
                writable: mount.writable(),
            })
            .collect();
        let image = arguments
            .iter()
            .rposition(|value| value == profile.image)
            .ok_or_else(|| anyhow::anyhow!(CONTAINER_CHANGED))?;
        Ok(Self {
            command,
            image: profile.image.into(),
            mounts,
            script: arguments[image + 1..].to_vec(),
        })
    }
    pub fn verify(&self, current: &Inspection) -> Result<()> {
        ensure!(
            valid_id(&current.id)
                && current.name.strip_prefix(INSPECTION_NAME_ROOT)
                    == Some(self.command.name.as_str())
                && current.config.image == self.image
                && current.config.cmd == self.script
                && current.config.labels.get(CONTAINER_OWNER_LABEL) == Some(&self.command.owner),
            CONTAINER_CHANGED
        );
        ensure!(
            current
                .mounts
                .iter()
                .all(|mount| mount.kind == CONTAINER_BIND_KIND
                    || (mount.kind == CONTAINER_TMPFS_KIND
                        && mount.destination == CONTAINER_TEMP_MOUNT)),
            CONTAINER_CHANGED
        );
        let actual = current
            .mounts
            .iter()
            .filter(|mount| mount.kind == CONTAINER_BIND_KIND)
            .collect::<Vec<_>>();
        ensure!(
            actual.len() == self.mounts.len()
                && self
                    .mounts
                    .iter()
                    .all(|expected| actual.contains(&expected)),
            CONTAINER_CHANGED
        );
        Ok(())
    }
}
pub(super) fn valid_id(value: &str) -> bool {
    value.len() == CONTAINER_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
