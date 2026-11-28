use super::{
    BROKER_READY_POLL_INTERVAL, BROKER_READY_TIMEOUT, BROKER_SUPERVISION_RECORD_VERSION,
    BrokerLaunchAuthority, BrokerSupervisionStore,
};
use crate::process::{
    BootIdentityRelation, ProcessIdentity, RetainedExactProcess, boot_id, boot_identity_relation,
};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::time::Instant;

const BROKER_IDENTITY_MISSING: &str = "supervised broker exact identity is missing";
const CONTAINMENT_DEADLINE_OVERFLOW: &str = "broker containment deadline overflow";
const CONTAINMENT_TIMEOUT: &str = "supervised broker exact containment timed out";
const RECORD_BOOT_INVALID: &str = "broker containment record belongs to another boot";
const RECORD_GENERATION_INVALID: &str = "broker containment generation is invalid";
const RECORD_IDENTITY_ALIAS: &str = "broker containment identities are aliased";
const RECORD_IDENTITY_INVALID: &str = "broker containment identity is invalid";
const RECORD_LAUNCH_INVALID: &str = "broker containment launch authority is invalid";
const RECORD_PHASE_INVALID: &str = "broker containment record phase is invalid";
const RECORD_PROVENANCE_INVALID: &str = "broker containment process provenance is invalid";
const RECORD_SELF_TARGET: &str = "broker containment cannot target the current process";
const RECORD_VERSION_INVALID: &str = "broker containment record version is invalid";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContainmentRecord {
    version: u16,
    launch_id: String,
    generation: String,
    boot_id: String,
    supervisor: PersistedIdentity,
    launcher: PersistedIdentity,
    broker: Option<PersistedIdentity>,
    instance_id: Option<uuid::Uuid>,
    broker_host_id: Option<String>,
    phase: ContainmentPhase,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ContainmentPhase {
    Intent,
    Active,
    Ready,
    Running,
    Terminating,
    Settled,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct PersistedIdentity {
    pid: u32,
    start_time: u64,
}

pub(super) fn contain(
    store: &BrokerSupervisionStore,
    name: &str,
    authority: BrokerLaunchAuthority,
) -> Result<()> {
    let deadline = Instant::now()
        .checked_add(BROKER_READY_TIMEOUT)
        .context(CONTAINMENT_DEADLINE_OVERFLOW)?;
    let record = store.read::<ContainmentRecord>(name)?;
    let identity = record.validate(authority)?;
    let Some(mut containment) = RetainedExactProcess::capture(identity)? else {
        return Ok(());
    };
    verify_process_provenance(identity, authority)?;
    store.verify_state()?;
    let _ = containment.begin_containment()?;
    loop {
        store.verify_state()?;
        if !containment.is_running()? {
            return Ok(());
        }
        let _ = containment.enforce_containment()?;
        let now = Instant::now();
        ensure!(now < deadline, CONTAINMENT_TIMEOUT);
        std::thread::sleep(
            deadline
                .saturating_duration_since(now)
                .min(BROKER_READY_POLL_INTERVAL),
        );
    }
}

fn verify_process_provenance(
    identity: ProcessIdentity,
    authority: BrokerLaunchAuthority,
) -> Result<()> {
    let observed = crate::agent::process_environment_value(
        identity.pid,
        crate::cli::constants::BROKER_SUPERVISION_AUTHORITY_ENVIRONMENT,
    )?;
    let expected = authority.to_string();
    ensure!(
        observed.as_deref() == Some(expected.as_str()),
        RECORD_PROVENANCE_INVALID
    );
    Ok(())
}

impl ContainmentRecord {
    fn validate(&self, authority: BrokerLaunchAuthority) -> Result<ProcessIdentity> {
        ensure!(
            self.version == BROKER_SUPERVISION_RECORD_VERSION,
            RECORD_VERSION_INVALID
        );
        ensure!(
            self.launch_id == authority.to_string(),
            RECORD_LAUNCH_INVALID
        );
        super::validate_broker_config_revision(&self.generation)
            .context(RECORD_GENERATION_INVALID)?;
        ensure!(
            boot_identity_relation(&self.boot_id, &boot_id()) == BootIdentityRelation::Same,
            RECORD_BOOT_INVALID
        );
        ensure!(
            self.supervisor.valid() && self.launcher.valid(),
            RECORD_IDENTITY_INVALID
        );
        let broker = self.broker.context(BROKER_IDENTITY_MISSING)?;
        ensure!(broker.valid(), RECORD_IDENTITY_INVALID);
        ensure!(
            broker != self.supervisor && broker != self.launcher,
            RECORD_IDENTITY_ALIAS
        );
        ensure!(self.shape_valid(), RECORD_PHASE_INVALID);
        if let Some(host_id) = self.broker_host_id.as_deref() {
            super::validate_broker_host_id(host_id)?;
        }
        let broker = ProcessIdentity::from(broker);
        ensure!(
            broker != ProcessIdentity::capture(std::process::id())?,
            RECORD_SELF_TARGET
        );
        Ok(broker)
    }

    fn shape_valid(&self) -> bool {
        let readiness =
            self.instance_id.is_some_and(|value| !value.is_nil()) && self.broker_host_id.is_some();
        let readiness_absent = self.instance_id.is_none() && self.broker_host_id.is_none();
        match self.phase {
            ContainmentPhase::Intent => false,
            ContainmentPhase::Active => readiness_absent,
            ContainmentPhase::Ready | ContainmentPhase::Running => readiness,
            ContainmentPhase::Terminating | ContainmentPhase::Settled => {
                readiness || readiness_absent
            }
        }
    }
}

impl PersistedIdentity {
    fn valid(self) -> bool {
        self.pid != u32::default() && self.start_time != u64::default()
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
