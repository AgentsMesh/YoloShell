use super::hook_policy::CodexHookPolicy;
use crate::agent::{FencedDescendants, ObservedProcess, ProcessSource};
use crate::codex_hooks::{CodexAdmissionContext, CodexNativeRequest};
use crate::process::ProcessIdentity;
use anyhow::{Result, ensure};
use std::sync::Arc;

#[path = "native_admission_evidence.rs"]
mod evidence;
#[path = "native_admission_lineage.rs"]
mod lineage;
#[path = "native_admission_proof.rs"]
mod proof;

const NATIVE_SCOPE: &str = "native Codex hook callback scope does not match its runtime";
pub(super) const NATIVE_PROCESS: &str =
    "native Codex hook callback process authority is unavailable";
const NATIVE_UNSTABLE: &str = "native Codex hook callback ancestry changed during admission";

pub(super) struct NativeAdmission {
    pub(super) agent: ObservedProcess,
    pub(super) context: CodexAdmissionContext,
    lineage: Vec<ObservedProcess>,
    /// `None` marks a SessionStart admitted before Codex wrote its rollout;
    /// later callbacks must supply authoritative transcript evidence.
    evidence: Option<crate::agent::codex::AuthoritativeCodexEvidence>,
}

pub(super) async fn admit(
    peer: ProcessIdentity,
    shell: ProcessIdentity,
    request: &CodexNativeRequest,
    policy: &CodexHookPolicy,
    source: Arc<dyn ProcessSource>,
    settlements: super::blocking_custody::BlockingSettlementCustody,
    generation: &super::cancellation::IngressCancellation,
) -> Result<NativeAdmission> {
    request.validate()?;
    ensure!(
        request.terminal_id == policy.terminal_id()
            && request.runtime_epoch == policy.runtime_epoch()
            && request.hook_executable == policy.hook_executable(),
        NATIVE_SCOPE
    );
    let initial = observe(Arc::clone(&source), shell, peer.pid, settlements.clone()).await?;
    let first = validate(peer, shell, request, policy, &initial)?;
    ensure!(!generation.is_cancelled(), NATIVE_UNSTABLE);
    tokio::task::yield_now().await;
    let current = observe(source, shell, peer.pid, settlements).await?;
    let second = validate(peer, shell, request, policy, &current)?;
    ensure!(
        first.agent == second.agent
            && first.lineage == second.lineage
            && first.context.cwd == second.context.cwd
            && first.context.data_home == second.context.data_home
            && same_evidence(&first.evidence, &second.evidence)
            && !generation.is_cancelled(),
        NATIVE_UNSTABLE
    );
    Ok(first)
}

/// Codex may write the rollout between the two observations. Absence on one
/// side is the expected provisional state, not an unstable admission.
fn same_evidence(
    first: &Option<crate::agent::codex::AuthoritativeCodexEvidence>,
    second: &Option<crate::agent::codex::AuthoritativeCodexEvidence>,
) -> bool {
    match (first, second) {
        (None, _) | (_, None) => true,
        (Some(first), Some(second)) => first.same_evidence(second),
    }
}

fn validate(
    peer: ProcessIdentity,
    shell: ProcessIdentity,
    request: &CodexNativeRequest,
    policy: &CodexHookPolicy,
    processes: &[ObservedProcess],
) -> Result<NativeAdmission> {
    let (agent, lineage) = lineage::capture(peer, shell, policy.hook_executable(), processes)?;
    let (context, evidence) = evidence::capture(&agent, request, policy)?;
    Ok(NativeAdmission {
        agent,
        context,
        lineage,
        evidence,
    })
}

pub(super) fn verify_live(
    source: &dyn ProcessSource,
    shell: ProcessIdentity,
    agent: ProcessIdentity,
    peer: ProcessIdentity,
    record: &crate::codex_hooks::CodexAdmissionRecord,
) -> Result<()> {
    observe_live(source, shell, agent, peer, record).map(|_| ())
}

fn observe_live(
    source: &dyn ProcessSource,
    shell: ProcessIdentity,
    agent: ProcessIdentity,
    peer: ProcessIdentity,
    record: &crate::codex_hooks::CodexAdmissionRecord,
) -> Result<ObservedProcess> {
    let processes = native_processes(source, shell, peer.pid)?;
    let (observed, _) = lineage::capture_exact(
        peer,
        shell,
        record.hook_executable().path(),
        agent,
        &processes,
    )?;
    ensure!(
        observed.identity_matches(agent.pid, agent.start_time)
            && agent.matches()
            && std::path::Path::new(&observed.executable) == record.request().real_executable,
        NATIVE_PROCESS
    );
    record.verify_native_processes(agent.pid, peer.pid)?;
    ensure!(peer.matches() && agent.matches(), NATIVE_PROCESS);
    Ok(observed)
}

impl super::runtime::CodexHookRuntime {
    pub(super) fn admit_record_process(
        &self,
        process: ProcessIdentity,
        peer: ProcessIdentity,
        record: &crate::codex_hooks::CodexAdmissionRecord,
    ) -> Result<(), super::error::CodexHookRejection> {
        verify_live(
            self.process_source.as_ref(),
            self.shell,
            process,
            peer,
            record,
        )
        .map_err(super::error::CodexHookRejection::Process)
    }
}

async fn observe(
    source: Arc<dyn ProcessSource>,
    shell: ProcessIdentity,
    peer_pid: u32,
    settlements: super::blocking_custody::BlockingSettlementCustody,
) -> Result<Vec<ObservedProcess>> {
    let settlement = settlements.admit();
    tokio::task::spawn_blocking(move || {
        let _settlement = settlement;
        native_processes(source.as_ref(), shell, peer_pid)
    })
    .await?
}

fn native_processes(
    source: &dyn ProcessSource,
    shell: ProcessIdentity,
    peer_pid: u32,
) -> Result<Vec<ObservedProcess>> {
    let FencedDescendants::Stable(processes) = source.fenced_callback_ancestry(shell, peer_pid)?
    else {
        anyhow::bail!(NATIVE_UNSTABLE);
    };
    #[cfg(target_os = "linux")]
    let processes = {
        let mut processes = processes;
        yolo_hook_lineage::normalize(
            peer_pid,
            shell,
            &mut processes,
            crate::agent::constants::MAX_PROCESS_ANCESTOR_DEPTH,
        )?;
        processes
    };
    Ok(processes)
}
