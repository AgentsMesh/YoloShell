use crate::agent::claude::ClaudeAdapter;
use crate::agent::{
    AgentAdapter, FencedDescendants, ObservedProcess, ProcessSource,
};
use crate::claude_hooks::ClaudeNativeScope;
use crate::process::ProcessIdentity;
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;

const INVALID: &str = "Claude native hook callback ancestry or PTY identity is invalid";
use crate::claude_hooks::NATIVE_CALLBACK_SHELLS as CALLBACK_SHELLS;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Admission {
    pub(super) agent: ObservedProcess,
    pub(super) peer: ProcessIdentity,
    lineage: Vec<ObservedProcess>,
}

pub(super) fn capture(
    peer_pid: u32,
    shell: ProcessIdentity,
    scope: ClaudeNativeScope,
    executable: &super::native_executable::HookExecutable,
    source: &dyn ProcessSource,
) -> Result<Admission> {
    let FencedDescendants::Stable(processes) = source.fenced_callback_ancestry(shell, peer_pid)? else {
        anyhow::bail!(INVALID);
    };
    #[cfg(target_os = "linux")]
    let processes = {
        let mut processes = processes;
        yolo_hook_lineage::normalize(
            peer_pid,
            shell,
            &mut processes,
            crate::claude_hooks::CLAUDE_NATIVE_MAX_ANCESTORS,
        )?;
        processes
    };
    let result = select(peer_pid, shell, scope, executable.path(), &processes)?;
    executable.verify_process(peer_pid)?;
    ensure!(
        result.peer.matches()
            && result
                .agent
                .identity_matches(scope.agent.pid, scope.agent.start_time),
        INVALID
    );
    Ok(result)
}

fn select(
    peer_pid: u32,
    shell: ProcessIdentity,
    scope: ClaudeNativeScope,
    executable: &std::path::Path,
    processes: &[ObservedProcess],
) -> Result<Admission> {
    let mut unique_pids = HashSet::new();
    ensure!(
        processes
            .iter()
            .all(|process| unique_pids.insert(process.pid)),
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
    // Claude runs hooks as background children; only the agent must own the
    // terminal foreground group, and the peer must descend through callback shells.
    loop {
        ensure!(
            lineage.len() < crate::claude_hooks::CLAUDE_NATIVE_MAX_ANCESTORS
                && seen.insert(current.pid),
            INVALID
        );
        if current.pid != peer_pid {
            if ClaudeAdapter.matches_process(current) {
                ensure!(
                    agent.is_none()
                        && current.identity_matches(scope.agent.pid, scope.agent.start_time),
                    INVALID
                );
                agent = Some(current.clone());
            } else if agent.is_none() {
                ensure!(
                    CALLBACK_SHELLS.contains(&current.executable.as_str()),
                    INVALID
                );
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
    ensure!(agent.is_foreground(), INVALID);
    for process in [&agent, peer] {
        let environment = process.environment.as_ref().context(INVALID)?;
        ensure!(
            environment.get(crate::agent::constants::YOLO_TERMINAL_ID_ENV)
                == Some(&scope.terminal_id.to_string())
                && environment.get(crate::agent::constants::YOLO_RUNTIME_EPOCH_ENV)
                    == Some(&scope.runtime_epoch.to_string()),
            INVALID
        );
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

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../unit_tests/claude_hook/native_admission_tests.rs"]
mod tests;
