use super::WorkerState;
use crate::model::{RuntimeRecord, TerminalManifest};
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const MANIFEST_INVALID: &str = "agent monitor durable manifest is invalid";
const RUNTIME_INVALID: &str = "agent monitor durable runtime authority changed";

pub(in crate::worker) struct AgentMonitorPublicationFence<'a> {
    guard: SessionDurableMutationGuard<'a>,
    runtime_path: PathBuf,
    runtime: RuntimeRecord,
    runtime_identity: FileIdentity,
    launch: crate::worker::agent_launch_fence::AgentLaunchFence,
    paths: &'a crate::paths::Paths,
}

pub(in crate::worker) fn acquire<'a>(
    state: &'a WorkerState,
    expected: &RuntimeRecord,
) -> Result<AgentMonitorPublicationFence<'a>> {
    let terminal_id = state.terminal_id();
    let guard = SessionDurableMutationGuard::acquire_participant(
        &state.paths,
        state.namespace.lease(),
        terminal_id,
    )?;
    capture(state, expected, guard)
}

pub(in crate::worker) fn try_acquire<'a>(
    state: &'a WorkerState,
    expected: &RuntimeRecord,
) -> Result<AgentMonitorPublicationFence<'a>> {
    let guard = SessionDurableMutationGuard::acquire(
        &state.paths,
        state.namespace.lease(),
        state.terminal_id(),
    )?;
    capture(state, expected, guard)
}

fn capture<'a>(
    state: &'a WorkerState,
    expected: &RuntimeRecord,
    guard: SessionDurableMutationGuard<'a>,
) -> Result<AgentMonitorPublicationFence<'a>> {
    let terminal_id = state.terminal_id();
    guard.require_no_pending_intent()?;
    let manifest: TerminalManifest = guard
        .read_optional_with_identity(&state.paths.manifest(terminal_id))?
        .map(|(value, _)| value)
        .context(MANIFEST_INVALID)?;
    ensure!(
        manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == terminal_id
            && !manifest.archived,
        MANIFEST_INVALID
    );
    let launch = crate::worker::agent_launch_fence::AgentLaunchFence::capture(
        state.namespace.lease(),
        &state.paths,
        expected,
    )?;
    let runtime_path = state.paths.runtime_record(terminal_id);
    let (current, runtime_identity): (RuntimeRecord, FileIdentity) = guard
        .read_optional_with_identity(&state.paths.runtime_record(terminal_id))?
        .context(RUNTIME_INVALID)?;
    ensure!(
        current.schema_version == crate::constants::RUNTIME_RECORD_SCHEMA_VERSION
            && current.terminal_id == terminal_id
            && current == *expected,
        RUNTIME_INVALID
    );
    guard.require_no_pending_intent()?;
    Ok(AgentMonitorPublicationFence {
        guard,
        runtime_path,
        runtime: current,
        runtime_identity,
        launch,
        paths: &state.paths,
    })
}

impl AgentMonitorPublicationFence<'_> {
    pub(in crate::worker) fn durable(&self) -> &SessionDurableMutationGuard<'_> {
        &self.guard
    }

    pub(in crate::worker) fn verify(&self) -> Result<()> {
        self.guard.require_no_pending_intent()?;
        let (current, identity): (RuntimeRecord, FileIdentity) = self
            .guard
            .read_optional_with_identity(&self.runtime_path)?
            .context(RUNTIME_INVALID)?;
        ensure!(
            current == self.runtime && identity == self.runtime_identity,
            RUNTIME_INVALID
        );
        self.launch
            .verify(self.guard.namespace(), self.paths, &current)?;
        self.guard.require_no_pending_intent()
    }
}
