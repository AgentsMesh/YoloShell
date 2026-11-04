use crate::{ChildLaunch, constants::*};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use yolo_process::ProcessIdentity;

mod recovery;
mod transitions;
pub use recovery::launch_id;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisionPhase {
    Intent,
    Active,
    Ready,
    Running,
    Terminating,
    Settled,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupervisionRecord {
    version: u16,
    pub(crate) launch_id: String,
    pub(crate) generation: String,
    boot_id: String,
    pub(crate) supervisor: PersistedIdentity,
    launcher: PersistedIdentity,
    pub(crate) broker: Option<PersistedIdentity>,
    pub(crate) instance_id: Option<uuid::Uuid>,
    pub(crate) broker_host_id: Option<String>,
    pub(crate) phase: SupervisionPhase,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PersistedIdentity {
    pid: u32,
    start_time: u64,
}

impl SupervisionRecord {
    pub fn intent(
        options: &ChildLaunch,
        supervisor: ProcessIdentity,
        launcher: ProcessIdentity,
    ) -> Result<Self> {
        Ok(Self {
            version: RECORD_VERSION,
            launch_id: launch_id(options),
            generation: options.config_revision().to_owned(),
            boot_id: yolo_process::boot_id(),
            supervisor: supervisor.into(),
            launcher: launcher.into(),
            broker: None,
            instance_id: None,
            broker_host_id: None,
            phase: SupervisionPhase::Intent,
        })
    }

    pub fn broker_identity(&self) -> Option<ProcessIdentity> {
        self.broker.map(Into::into)
    }

    pub fn instance_id(&self) -> Option<uuid::Uuid> {
        self.instance_id
    }

    pub fn matches_status(&self, status: &yolo_remote_broker::BrokerStatus) -> bool {
        self.phase == SupervisionPhase::Ready
            && self.instance_id == Some(status.instance_id)
            && self.broker_host_id.as_deref() == Some(status.host_id.as_str())
            && self.generation == status.config_revision
    }

    pub fn same_custody(&self, other: &Self) -> bool {
        self.shape_valid()
            && other.shape_valid()
            && self.version == other.version
            && self.launch_id == other.launch_id
            && self.generation == other.generation
            && self.boot_id == other.boot_id
            && self.supervisor == other.supervisor
            && self.launcher == other.launcher
            && self.broker == other.broker
            && self
                .instance_id
                .is_none_or(|expected| Some(expected) == other.instance_id)
            && self
                .broker_host_id
                .as_ref()
                .is_none_or(|expected| Some(expected) == other.broker_host_id.as_ref())
    }

    pub fn launcher_identity(&self) -> ProcessIdentity {
        self.launcher.into()
    }

    #[cfg(test)]
    pub(crate) fn set_phase(&mut self, phase: SupervisionPhase) {
        self.phase = phase;
    }

    pub fn verify(
        &self,
        options: &ChildLaunch,
        supervisor: ProcessIdentity,
        launcher: ProcessIdentity,
    ) -> Result<()> {
        ensure!(self.version == RECORD_VERSION, RECORD_VERSION_DETAIL);
        ensure!(self.launch_id == launch_id(options), RECORD_LAUNCH_DETAIL);
        ensure!(
            self.generation == options.config_revision(),
            RECORD_GENERATION_DETAIL
        );
        ensure!(self.boot_id == yolo_process::boot_id(), RECORD_BOOT_DETAIL);
        ensure!(
            self.supervisor == supervisor.into(),
            RECORD_SUPERVISOR_DETAIL
        );
        ensure!(self.launcher == launcher.into(), RECORD_LAUNCHER_DETAIL);
        ensure!(self.shape_valid(), RECORD_PHASE_DETAIL);
        Ok(())
    }

    fn shape_valid(&self) -> bool {
        let broker = self.broker.is_some();
        let readiness =
            self.instance_id.is_some_and(|value| !value.is_nil()) && self.broker_host_id.is_some();
        let readiness_absent = self.instance_id.is_none() && self.broker_host_id.is_none();
        match self.phase {
            SupervisionPhase::Intent => !broker && readiness_absent,
            SupervisionPhase::Active => broker && readiness_absent,
            SupervisionPhase::Ready | SupervisionPhase::Running => broker && readiness,
            SupervisionPhase::Terminating | SupervisionPhase::Settled => {
                broker && (readiness || readiness_absent)
            }
        }
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/record.rs"]
mod tests;
