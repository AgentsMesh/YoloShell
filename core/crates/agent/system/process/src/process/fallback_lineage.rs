use super::super::constants::MAX_PROCESS_ANCESTOR_DEPTH;
use super::fallback_observation::{ProcessIdentity, ProcessIdentityApi, ProcessRow};
use std::collections::{HashMap, HashSet};

pub(super) fn identity_index(rows: &[ProcessRow]) -> Option<HashMap<u32, ProcessIdentity>> {
    let mut identities = HashMap::with_capacity(rows.len());
    for row in rows {
        let identity = row.inventory_identity.as_ref()?;
        if identity.pid != row.pid
            || identity.parent_pid != row.parent_pid
            || identities.insert(row.pid, identity.clone()).is_some()
        {
            return None;
        }
    }
    Some(identities)
}

pub(super) fn is_foreground_candidate(process: &ProcessRow, root: &ProcessIdentity) -> bool {
    process.pid != root.pid
        && root.foreground_group > 0
        && process.process_group == root.foreground_group
        && process.foreground_group == root.foreground_group
}

pub(super) fn captured_lineage(
    identities: &HashMap<u32, ProcessIdentity>,
    leaf_pid: u32,
    root: &ProcessIdentity,
) -> Option<Vec<ProcessIdentity>> {
    if leaf_pid == root.pid {
        return None;
    }
    let mut lineage = Vec::new();
    let mut visited = HashSet::new();
    let mut current = leaf_pid;
    loop {
        if lineage.len() >= MAX_PROCESS_ANCESTOR_DEPTH || !visited.insert(current) {
            return None;
        }
        let identity = identities.get(&current)?.clone();
        lineage.push(identity.clone());
        if current == root.pid {
            return (identity == *root).then_some(lineage);
        }
        current = identity.parent_pid;
    }
}

pub(super) fn lineage_is_stable(
    api: &impl ProcessIdentityApi,
    expected: &[ProcessIdentity],
) -> bool {
    !expected.is_empty()
        && expected
            .windows(2)
            .all(|pair| pair[0].parent_pid == pair[1].pid)
        && expected
            .iter()
            .all(|identity| api.identity(identity.pid).as_ref() == Some(identity))
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_lineage_tests.rs"]
mod tests;
