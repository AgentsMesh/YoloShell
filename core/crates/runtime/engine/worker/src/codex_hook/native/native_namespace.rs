use super::CodexHookContext;
use crate::model::{RuntimeRecord, TerminalManifest};
use crate::paths::durable_mutation::SessionDurableMutationGuard;
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};

const NATIVE_NAMESPACE: &str = "native Codex callback lost its exact Worker runtime namespace";
const NATIVE_RUNTIME_READ: &str = "read native Codex callback runtime authority";
const NATIVE_MANIFEST_READ: &str = "read native Codex callback session manifest";
const NATIVE_LAUNCH_CAPTURE: &str = "capture native Codex callback launch authority";
const NATIVE_LAUNCH_VERIFY: &str = "verify native Codex callback launch authority";

pub(super) struct NativeNamespaceFence<'a> {
    transaction: &'a CodexHookContext,
    guard: SessionDurableMutationGuard<'a>,
    runtime: RuntimeRecord,
    runtime_identity: FileIdentity,
    launch: crate::worker::agent_launch_fence::AgentLaunchFence,
}

pub(super) fn acquire(transaction: &CodexHookContext) -> Result<NativeNamespaceFence<'_>> {
    let namespace = transaction.native_namespace.as_ref();
    let terminal_id = transaction.policy.terminal_id();
    let guard = SessionDurableMutationGuard::acquire_participant(
        &transaction.paths,
        namespace,
        terminal_id,
    )?;
    guard.require_no_pending_intent()?;
    let (runtime, runtime_identity): (RuntimeRecord, _) = namespace
        .read_json_with_identity(&transaction.paths.runtime_record(terminal_id))
        .context(NATIVE_RUNTIME_READ)?;
    let worker = crate::process::ProcessIdentity::capture(std::process::id())?;
    ensure!(
        runtime.schema_version == crate::constants::RUNTIME_RECORD_SCHEMA_VERSION
            && runtime.terminal_id == terminal_id
            && runtime.runtime_epoch == transaction.policy.runtime_epoch()
            && runtime.child_pid == transaction.shell.pid
            && runtime.child_start_time == transaction.shell.start_time
            && runtime.worker_pid == worker.pid
            && runtime.worker_start_time == worker.start_time,
        NATIVE_NAMESPACE
    );
    let manifest: TerminalManifest = namespace
        .read_json(&transaction.paths.manifest(terminal_id))
        .context(NATIVE_MANIFEST_READ)?;
    ensure!(
        manifest.schema_version == crate::constants::SESSION_MANIFEST_SCHEMA_VERSION
            && manifest.terminal_id == terminal_id
            && !manifest.archived,
        NATIVE_NAMESPACE
    );
    let launch = crate::worker::agent_launch_fence::AgentLaunchFence::capture(
        namespace,
        &transaction.paths,
        &runtime,
    )
    .context(NATIVE_LAUNCH_CAPTURE)?;
    guard.require_no_pending_intent()?;
    Ok(NativeNamespaceFence {
        transaction,
        guard,
        runtime,
        runtime_identity,
        launch,
    })
}

impl NativeNamespaceFence<'_> {
    pub(super) fn verify(&self) -> Result<()> {
        self.guard.require_no_pending_intent()?;
        let namespace = self.guard.namespace();
        let paths = &self.transaction.paths;
        let current: (RuntimeRecord, FileIdentity) = namespace
            .read_json_with_identity(&paths.runtime_record(self.runtime.terminal_id))
            .context(NATIVE_RUNTIME_READ)?;
        ensure!(
            current.0 == self.runtime && current.1 == self.runtime_identity,
            NATIVE_NAMESPACE
        );
        self.launch
            .verify(namespace, paths, &self.runtime)
            .context(NATIVE_LAUNCH_VERIFY)?;
        self.guard.require_no_pending_intent()
    }
}
