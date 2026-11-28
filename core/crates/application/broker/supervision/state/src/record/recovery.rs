use super::{
    PersistedIdentity, RECORD_LAUNCH_DETAIL, RECORD_PHASE_DETAIL, RECORD_VERSION,
    RECORD_VERSION_DETAIL, SupervisionRecord,
};
use crate::ChildLaunch;
use anyhow::{Result, ensure};
use yolo_process::ProcessIdentity;
use yolo_remote_broker::{
    BrokerLaunchAuthority, validate_broker_config_revision, validate_broker_host_id,
};

const RECORD_IDENTITY_DETAIL: &str = "broker supervision persisted identity is invalid";

impl SupervisionRecord {
    pub fn verify_recovery(&self, expected_launch_id: &str) -> Result<()> {
        ensure!(self.version == RECORD_VERSION, RECORD_VERSION_DETAIL);
        ensure!(self.launch_id == expected_launch_id, RECORD_LAUNCH_DETAIL);
        let authority = self.launch_id.parse::<BrokerLaunchAuthority>()?;
        ensure!(
            authority.to_string() == self.launch_id,
            RECORD_LAUNCH_DETAIL
        );
        validate_broker_config_revision(&self.generation)?;
        if let Some(host_id) = self.broker_host_id.as_deref() {
            validate_broker_host_id(host_id)?;
        }
        ensure!(identity_valid(self.supervisor), RECORD_IDENTITY_DETAIL);
        ensure!(identity_valid(self.launcher), RECORD_IDENTITY_DETAIL);
        ensure!(
            self.broker.is_none_or(identity_valid),
            RECORD_IDENTITY_DETAIL
        );
        ensure!(self.shape_valid(), RECORD_PHASE_DETAIL);
        Ok(())
    }

    pub fn recovery_boot_id(&self) -> &str {
        &self.boot_id
    }

    pub fn supervisor_identity(&self) -> ProcessIdentity {
        self.supervisor.into()
    }
}

impl From<ProcessIdentity> for PersistedIdentity {
    fn from(identity: ProcessIdentity) -> Self {
        Self {
            pid: identity.pid,
            start_time: identity.start_time,
        }
    }
}

impl From<PersistedIdentity> for ProcessIdentity {
    fn from(identity: PersistedIdentity) -> Self {
        Self {
            pid: identity.pid,
            start_time: identity.start_time,
        }
    }
}

pub fn launch_id(options: &ChildLaunch) -> String {
    options.authority().to_string()
}

fn identity_valid(identity: PersistedIdentity) -> bool {
    identity.pid != u32::default() && identity.start_time != u64::default()
}
