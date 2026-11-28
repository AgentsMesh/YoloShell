use super::constants::INVALID;
use super::contract::HookScope;
use super::executable::HookExecutable;
use super::profile::{CALLBACK_SHELLS, HookAncestry, HookIngressProfile};
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
use yolo_agent_adapter_contract::AgentAdapter;
use yolo_agent_contract::ObservedProcess;
use yolo_agent_process::{FencedDescendants, ProcessSource};
use yolo_process::ProcessIdentity;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Admission {
    pub agent: ObservedProcess,
    pub peer: ProcessIdentity,
    lineage: Vec<ObservedProcess>,
}

pub fn capture(
    peer_pid: u32,
    profile: &HookIngressProfile,
    shell: ProcessIdentity,
    scope: HookScope,
    executable: &HookExecutable,
    source: &dyn ProcessSource,
    adapter: &dyn AgentAdapter,
) -> Result<Admission> {
    let fenced = match profile.ancestry {
        HookAncestry::DetachedSession => {
            source.fenced_callback_ancestry(shell, peer_pid)?
        }
        HookAncestry::ForegroundGroup => source.fenced_callback_lineage(shell, peer_pid)?,
    };
    let FencedDescendants::Stable(processes) = fenced else {
        anyhow::bail!(INVALID);
    };
    let result = select(peer_pid, profile, shell, scope, executable.path(), &processes, adapter)?;
    executable.verify_process(peer_pid)?;
    ensure!(
        result.peer.matches() && result.agent.identity_matches(scope.agent.pid, scope.agent.start_time),
        INVALID
    );
    Ok(result)
}

fn require_fence(process: &ObservedProcess, scope: HookScope) -> Result<()> {
    let environment = process.environment.as_ref().context(INVALID)?;
    ensure!(
        environment.get(yolo_agent_core::constants::YOLO_TERMINAL_ID_ENV)
            == Some(&scope.terminal_id.to_string())
            && environment.get(yolo_agent_core::constants::YOLO_RUNTIME_EPOCH_ENV)
                == Some(&scope.runtime_epoch.to_string()),
        INVALID
    );
    Ok(())
}

pub(super) fn select(
    peer_pid: u32,
    profile: &HookIngressProfile,
    shell: ProcessIdentity,
    scope: HookScope,
    executable: &std::path::Path,
    processes: &[ObservedProcess],
    adapter: &dyn AgentAdapter,
) -> Result<Admission> {
    let mut unique_pids = HashSet::new();
    ensure!(
        processes.iter().all(|process| unique_pids.insert(process.pid)),
        INVALID
    );
    let peer = processes
        .iter()
        .find(|process| process.pid == peer_pid)
        .context(INVALID)?;
    ensure!(
        std::path::Path::new(&peer.executable) == executable
            && peer.start_time != u64::default()
            && peer.pid != shell.pid,
        INVALID
    );
    let mut current = peer;
    let mut lineage = Vec::new();
    let mut seen = HashSet::new();
    let mut agent = None;
    loop {
        ensure!(
            lineage.len() < profile.max_ancestors
                && seen.insert(current.pid)
                && current.start_time != u64::default()
                && current.pid != shell.pid,
            INVALID
        );
        if current.pid != peer_pid {
            if current.identity_matches(scope.agent.pid, scope.agent.start_time) {
                // The claimed agent may sit above further processes that also
                // match the adapter (a provider launcher such as Gemini's
                // flag-bearing relaunch); the fence is the claimed identity.
                ensure!(agent.is_none() && adapter.matches_process(current), INVALID);
                agent = Some(current.clone());
            } else if agent.is_none() && !adapter.matches_process(current) {
                ensure!(CALLBACK_SHELLS.contains(&current.executable.as_str()), INVALID);
            }
        }
        lineage.push(current.clone());
        if current.parent_pid == shell.pid {
            break;
        }
        current = processes
            .iter()
            .find(|process| process.pid == current.parent_pid)
            .context(INVALID)?;
    }
    let agent = agent.context(INVALID)?;
    require_fence(&agent, scope)?;
    if profile.require_peer_environment {
        require_fence(peer, scope)?;
    }
    Ok(Admission {
        agent,
        peer: ProcessIdentity {
            pid: peer.pid,
            start_time: peer.start_time,
        },
        lineage,
    })
}
