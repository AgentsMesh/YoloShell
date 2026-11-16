use crate::model::RuntimeRecord;
use crate::paths::identity::FileIdentity;
use crate::worker::{WorkerState, lock_recover};
use anyhow::{Result, ensure};

const INVALID: &str = "native callback lost its exact Worker runtime";

pub(in crate::worker) struct RuntimeFence {
    record: RuntimeRecord,
    identity: FileIdentity,
    launch: crate::worker::agent_launch_fence::AgentLaunchFence,
}

impl RuntimeFence {
    pub(in crate::worker) fn capture(state: &WorkerState) -> Result<Self> {
        let path = state.paths.runtime_record(state.terminal_id());
        let (record, identity) = state.namespace.read_json_with_identity(&path)?;
        let launch = crate::worker::agent_launch_fence::AgentLaunchFence::capture(
            state.namespace.lease(),
            &state.paths,
            &record,
        )?;
        let fence = Self {
            record,
            identity,
            launch,
        };
        fence.verify(state)?;
        Ok(fence)
    }

    pub(in crate::worker) fn verify(&self, state: &WorkerState) -> Result<()> {
        state.namespace.verify()?;
        let path = state.paths.runtime_record(state.terminal_id());
        let (record, identity): (RuntimeRecord, _) =
            state.namespace.read_json_with_identity(&path)?;
        ensure!(
            record.schema_version == crate::constants::RUNTIME_RECORD_SCHEMA_VERSION
                && record == self.record
                && identity == self.identity
                && *lock_recover(&state.record) == self.record
                && !state.has_termination_intent(),
            INVALID
        );
        self.launch
            .verify(state.namespace.lease(), &state.paths, &record)?;
        state.namespace.verify()
    }
}
