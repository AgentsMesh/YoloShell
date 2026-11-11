use crate::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentSnapshot, AgentSnapshotFields, Capability, Freshness,
    Reachability,
};
use yolo_hub_identity::{AgentId, ExecutionId, HubId, RuntimeEpoch, TerminalId};
use yolo_paths::identity::DurableObjectIdentity;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Subject {
    pub terminal_id: TerminalId,
    pub kind: String,
    pub native_reference: String,
    pub data_home: String,
    pub evidence_path: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Process {
    pub pid: u32,
    pub start_time: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub subject: Subject,
    pub runtime: RuntimeEpoch,
    pub process: Option<Process>,
    pub detected_at_ms: i64,
    pub updated_at_ms: i64,
    pub source_exited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_witness: Option<crate::control::ControlWitness>,
    #[serde(deserialize_with = "migrating_snapshot")]
    pub snapshot: AgentSnapshot,
}

/// Record schema 4 only ever carried snapshot schema 2. Deserialization runs
/// before `Record::migrate`, so that historical pairing is normalized here.
fn migrating_snapshot<'de, D>(deserializer: D) -> Result<AgentSnapshot, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut fields = AgentSnapshotFields::deserialize(deserializer)?;
    if fields.schema_version == RECORD_4_SNAPSHOT_SCHEMA {
        fields.schema_version = AGENT_PROTOCOL_SCHEMA_VERSION;
    }
    AgentSnapshot::new(fields).map_err(serde::de::Error::custom)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub schema: u16,
    pub hub: HubId,
    pub directory: DurableObjectIdentity,
    pub entries: BTreeMap<AgentId, Entry>,
    /// Most recently associated logical agent; its execution may already be historical.
    pub latest: BTreeMap<TerminalId, AgentId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub retired_controls: BTreeMap<ExecutionId, crate::control::ControlWitness>,
}

fn unavailable(fields: &mut AgentSnapshotFields, retained: bool) {
    crate::control::capability(fields, retained, false, false);
    // An unreachable runtime cannot answer an interaction; keeping the
    // reference would also violate the record's answerability invariant.
    fields.interaction = None;
    fields.capabilities.respond_interaction = Capability::Unsupported {};
}

impl Record {
    /// Move a loaded previous-schema record to the current schema. Snapshots
    /// keep their identity and witnesses; only the schema marker changes.
    pub fn migrate(&mut self) -> anyhow::Result<()> {
        if self.schema == SCHEMA_VERSION {
            return Ok(());
        }
        for entry in self.entries.values_mut() {
            let mut fields = entry.snapshot.fields().clone();
            fields.schema_version = AGENT_PROTOCOL_SCHEMA_VERSION;
            entry.snapshot = AgentSnapshot::new(fields)?;
        }
        self.schema = SCHEMA_VERSION;
        Ok(())
    }

    pub fn validate(&self, hub: HubId, directory: DurableObjectIdentity) -> Result<()> {
        ensure!(
            self.schema == SCHEMA_VERSION || self.schema == PREVIOUS_SCHEMA_VERSION,
            INVALID_RECORD
        );
        ensure!(self.hub == hub, WRONG_ORIGIN);
        ensure!(self.directory == directory, REPLACED_DIRECTORY);
        ensure!(self.entries.len() <= MAX_AGENT_RECORDS, RECORD_LIMIT);
        let mut executions = BTreeSet::new();
        for (id, entry) in &self.entries {
            let snapshot = entry.snapshot.fields();
            let target = snapshot.cursor.target();
            ensure!(
                target.address.hub_id == hub && target.address.agent_id == *id,
                INVALID_RECORD
            );
            ensure!(
                snapshot.terminal.address.terminal_id == entry.subject.terminal_id
                    && snapshot.terminal.runtime_epoch == entry.runtime,
                INVALID_RECORD
            );
            ensure!(
                snapshot.provider.kind() == entry.subject.kind
                    && snapshot.provider.native_session_reference()
                        == Some(entry.subject.native_reference.as_str()),
                INVALID_RECORD
            );
            ensure!(
                entry.detected_at_ms >= 0 && entry.updated_at_ms >= entry.detected_at_ms,
                INVALID_RECORD
            );
            ensure!(
                snapshot.observed_unix_ms == entry.updated_at_ms as u64,
                INVALID_RECORD
            );
            for path in [&entry.subject.data_home, &entry.subject.evidence_path] {
                ensure!(
                    path.len() <= MAX_PRIVATE_METADATA_BYTES
                        && std::path::Path::new(path).is_absolute()
                        && !path.chars().any(char::is_control),
                    INVALID_RECORD
                );
            }
            ensure!(
                entry.process.is_none_or(|process| process.pid
                    >= yolo_agent_adapter_contract::constants::MIN_PROCESS_PID
                    && process.start_time != 0),
                INVALID_RECORD
            );
            ensure!(executions.insert(target.execution_id), INVALID_RECORD);
            crate::control_record::validate_entry(entry)?;
        }
        for (session, id) in &self.latest {
            ensure!(
                self.entries
                    .get(id)
                    .is_some_and(|entry| entry.subject.terminal_id == *session),
                INVALID_RECORD
            );
        }
        crate::control_record::validate_retired(self)?;
        Ok(())
    }

    pub fn invalidate(&mut self, session: Option<TerminalId>) -> Result<()> {
        for entry in self
            .entries
            .values_mut()
            .filter(|entry| session.is_none_or(|session| entry.subject.terminal_id == session))
        {
            let mut fields = entry.snapshot.fields().clone();
            fields.freshness = Freshness::Stale;
            fields.reachability = Reachability::Unreachable;
            unavailable(&mut fields, entry.control_witness.is_some());
            entry.snapshot = crate::observation::revise(&entry.snapshot, fields)?;
        }
        Ok(())
    }
}
