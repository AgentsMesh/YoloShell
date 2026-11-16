use super::NATIVE_PROCESS;
use crate::agent::{AgentAdapter, AgentKind, ObservedProcess};
use crate::process::ProcessIdentity;
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::codex_hooks::NATIVE_CALLBACK_SHELLS as CALLBACK_SHELLS;
#[path = "native_admission_groups.rs"]
mod groups;

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/native/native_admission_groups_tests.rs"]
mod group_tests;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/codex_hook/native/native_admission_lineage.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/native/native_admission_lineage_tests.rs"]
mod tests;

pub(super) fn capture(
    peer: ProcessIdentity,
    shell: ProcessIdentity,
    hook: &Path,
    processes: &[ObservedProcess],
) -> Result<(ObservedProcess, Vec<ObservedProcess>)> {
    capture_matching(peer, shell, hook, None, processes)
}

pub(super) fn capture_exact(
    peer: ProcessIdentity,
    shell: ProcessIdentity,
    hook: &Path,
    agent: ProcessIdentity,
    processes: &[ObservedProcess],
) -> Result<(ObservedProcess, Vec<ObservedProcess>)> {
    capture_matching(peer, shell, hook, Some(agent), processes)
}

fn capture_matching(
    peer: ProcessIdentity,
    shell: ProcessIdentity,
    hook: &Path,
    expected: Option<ProcessIdentity>,
    processes: &[ObservedProcess],
) -> Result<(ObservedProcess, Vec<ObservedProcess>)> {
    ensure!(
        peer.pid != shell.pid && peer.start_time != u64::default(),
        NATIVE_PROCESS
    );
    let mut index = HashMap::new();
    for process in processes {
        ensure!(index.insert(process.pid, process).is_none(), NATIVE_PROCESS);
    }
    let callback = index.get(&peer.pid).copied().context(NATIVE_PROCESS)?;
    ensure!(
        callback.identity_matches(peer.pid, peer.start_time)
            && Path::new(&callback.executable) == hook
            && callback.process_group > i32::default()
            && callback.foreground_group > i32::default(),
        NATIVE_PROCESS
    );
    let adapter = crate::agent::codex::CodexAdapter;
    let mut current = callback;
    let mut visited = HashSet::new();
    let mut chain = Vec::new();
    let mut agent = None;
    loop {
        ensure!(
            chain.len() < crate::agent::constants::MAX_PROCESS_ANCESTOR_DEPTH
                && visited.insert(current.pid)
                && current.foreground_group == callback.foreground_group,
            NATIVE_PROCESS
        );
        if current.pid != peer.pid && agent.is_none() {
            let matches = expected.map_or_else(
                || adapter.matches_process(current),
                |expected| current.identity_matches(expected.pid, expected.start_time),
            );
            if matches {
                groups::provider_boundary(current, callback, &chain)?;
                ensure!(
                    expected.is_some() || adapter.process_session_arguments_valid(current),
                    NATIVE_PROCESS
                );
                agent = Some(current.clone());
            } else {
                ensure!(
                    CALLBACK_SHELLS.contains(&current.executable.as_str())
                        && current.process_group == callback.process_group,
                    NATIVE_PROCESS
                );
            }
        }
        if agent.is_some() {
            ensure!(current.is_foreground(), NATIVE_PROCESS);
        }
        chain.push(current.clone());
        if current.parent_pid == shell.pid {
            break;
        }
        current = index
            .get(&current.parent_pid)
            .copied()
            .context(NATIVE_PROCESS)?;
    }
    Ok((agent.context(NATIVE_PROCESS)?, chain))
}
