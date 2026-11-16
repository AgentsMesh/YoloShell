use super::constants::{CODEX_PROCESS_ERROR, CODEX_RUNTIME_ERROR, CODEX_SESSION_ERROR};
use super::error::CodexHookRejection;
use super::runtime::{CodexHookRuntime, CodexIngressEvidence};
use crate::agent::FencedDescendants;
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::CodexExecutableKind;
use std::collections::HashMap;
use std::path::Path;

impl CodexHookRuntime {
    pub(super) fn admit_event(
        &self,
        event: &HookEnvelope,
        evidence: &CodexIngressEvidence<'_>,
    ) -> Result<(), CodexHookRejection> {
        self.validate_event_scope(event)?;
        match evidence {
            #[cfg(test)]
            CodexIngressEvidence::TestPeer(peer_pid) => {
                let process = event.identity.process;
                self.admit_test_process(
                    crate::process::ProcessIdentity {
                        pid: process.pid,
                        start_time: process.start_time,
                    },
                    *peer_pid,
                )
            }
            CodexIngressEvidence::VerifiedDurableOccurrence(proof) => proof.verify(self, event),
            CodexIngressEvidence::VerifiedRetirementOccurrence(proof) => proof.verify(self, event),
        }
    }

    pub(super) fn validate_event_scope(
        &self,
        event: &HookEnvelope,
    ) -> Result<(), CodexHookRejection> {
        let identity = &event.identity;
        identity.validate().map_err(CodexHookRejection::Violation)?;
        if identity.terminal_id != self.terminal_id {
            return Err(identity_error(CODEX_SESSION_ERROR));
        }
        if identity.runtime_epoch != self.runtime_epoch {
            return Err(identity_error(CODEX_RUNTIME_ERROR));
        }
        Ok(())
    }

    pub(super) fn admit_live_process(
        &self,
        process: crate::process::ProcessIdentity,
        peer: crate::process::ProcessIdentity,
        hook_executable: &crate::codex_hooks::CodexExecutableAttestation,
    ) -> Result<(), CodexHookRejection> {
        admit_live_process_source(
            self.process_source.as_ref(),
            self.shell,
            process,
            peer,
            hook_executable,
        )
    }

    #[cfg(test)]
    fn admit_test_process(
        &self,
        process: crate::process::ProcessIdentity,
        peer_pid: u32,
    ) -> Result<(), CodexHookRejection> {
        if peer_pid != process.pid || !process.matches() {
            return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
        }
        let observed = self
            .process_source
            .fenced_descendants(self.shell)
            .map_err(process_error)?;
        let FencedDescendants::Stable(processes) = observed else {
            return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
        };
        let exact = processes
            .iter()
            .filter(|candidate| candidate.pid == process.pid)
            .collect::<Vec<_>>();
        if exact.len() != 1 || !exact[0].identity_matches(process.pid, process.start_time) {
            return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
        }
        Ok(())
    }
}

pub(super) fn admit_process_source(
    process_source: &dyn crate::agent::ProcessSource,
    shell: crate::process::ProcessIdentity,
    process: crate::process::ProcessIdentity,
    peer_pid: u32,
) -> Result<(), CodexHookRejection> {
    if peer_pid != process.pid || !process.matches() {
        return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
    }
    let observed = process_source
        .fenced_descendants(shell)
        .map_err(process_error)?;
    let FencedDescendants::Stable(processes) = observed else {
        return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
    };
    let exact = processes
        .iter()
        .filter(|candidate| candidate.pid == process.pid)
        .collect::<Vec<_>>();
    if exact.len() != 1 || !exact[0].identity_matches(process.pid, process.start_time) {
        return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
    }
    Ok(())
}

pub(super) fn admit_live_process_source(
    process_source: &dyn crate::agent::ProcessSource,
    shell: crate::process::ProcessIdentity,
    process: crate::process::ProcessIdentity,
    peer: crate::process::ProcessIdentity,
    hook_executable: &crate::codex_hooks::CodexExecutableAttestation,
) -> Result<(), CodexHookRejection> {
    if !process.matches() {
        return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
    }
    let observed = process_source
        .fenced_descendants(shell)
        .map_err(process_error)?;
    let FencedDescendants::Stable(descendants) = observed else {
        return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
    };
    let admitted = peer_is_direct_hook(peer, process, hook_executable.path(), &descendants);
    #[cfg(target_os = "macos")]
    hook_executable
        .verify_mapped_process(peer.pid, CodexExecutableKind::Runtime)
        .map_err(process_error)?;
    if !admitted || !process.matches() {
        return Err(process_error(anyhow::anyhow!(CODEX_PROCESS_ERROR)));
    }
    Ok(())
}

pub(super) fn peer_is_direct_hook(
    peer: crate::process::ProcessIdentity,
    agent: crate::process::ProcessIdentity,
    hook_executable: &Path,
    processes: &[crate::agent::ObservedProcess],
) -> bool {
    if peer.pid < crate::agent::constants::MIN_PROCESS_PID || peer.start_time == u64::default() {
        return false;
    }
    let Some(index) = unique_processes(processes) else {
        return false;
    };
    let Some(agent_process) = index.get(&agent.pid) else {
        return false;
    };
    if !agent_process.identity_matches(agent.pid, agent.start_time) {
        return false;
    }
    let Some(observed_peer) = index.get(&peer.pid) else {
        return false;
    };
    peer.pid != agent.pid
        && observed_peer.identity_matches(peer.pid, peer.start_time)
        && observed_peer.parent_pid == agent.pid
        && agent_process.is_foreground()
        && observed_peer.is_foreground()
        && observed_peer.process_group == agent_process.process_group
        && observed_peer.foreground_group == agent_process.foreground_group
        && Path::new(&observed_peer.executable) == hook_executable
}

fn unique_processes(
    processes: &[crate::agent::ObservedProcess],
) -> Option<HashMap<u32, &crate::agent::ObservedProcess>> {
    let mut index = HashMap::with_capacity(processes.len());
    for process in processes {
        if index.insert(process.pid, process).is_some() {
            return None;
        }
    }
    Some(index)
}

fn identity_error(detail: &'static str) -> CodexHookRejection {
    CodexHookRejection::Identity(detail)
}

fn process_error(error: anyhow::Error) -> CodexHookRejection {
    CodexHookRejection::Process(error)
}
