use crate::process::macos_native;
use anyhow::Result;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct ExternalProcess {
    lineage: u64,
    parent_lineage: u64,
    parent_pid: u32,
    start_time: u64,
}

pub(super) struct ExternalAncestry {
    minimum_start_time: u64,
    root_lineage: u64,
    resolved: HashMap<(u32, u64), bool>,
}

impl ExternalAncestry {
    pub(super) fn new(minimum_start_time: u64, root_lineage: u64) -> Self {
        Self {
            minimum_start_time,
            root_lineage,
            resolved: HashMap::new(),
        }
    }

    pub(super) fn could_inherit(&mut self, parent_pid: u32, parent_lineage: u64) -> bool {
        resolve(
            parent_pid,
            parent_lineage,
            self.minimum_start_time,
            self.root_lineage,
            &mut self.resolved,
            native_process,
        )
    }
}

fn native_process(pid: u32) -> Result<Option<ExternalProcess>> {
    Ok(
        macos_native::try_kernel_process_metadata(pid)?.map(|metadata| ExternalProcess {
            lineage: metadata.unique_id,
            parent_lineage: metadata.parent_unique_id,
            parent_pid: metadata.parent_pid,
            start_time: metadata.start_time,
        }),
    )
}

fn resolve<Q>(
    mut pid: u32,
    mut lineage: u64,
    minimum_start_time: u64,
    root_lineage: u64,
    cache: &mut HashMap<(u32, u64), bool>,
    mut query: Q,
) -> bool
where
    Q: FnMut(u32) -> Result<Option<ExternalProcess>>,
{
    let mut path = Vec::new();
    let mut visited = HashSet::new();
    let outcome = loop {
        let key = (pid, lineage);
        if lineage == root_lineage {
            break true;
        }
        if let Some(resolved) = cache.get(&key).copied() {
            break resolved;
        }
        if !visited.insert(key) {
            break true;
        }
        path.push(key);
        let Ok(Some(process)) = query(pid) else {
            break true;
        };
        if process.lineage != lineage {
            break true;
        }
        if process.start_time < minimum_start_time {
            break false;
        }
        pid = process.parent_pid;
        lineage = process.parent_lineage;
    };
    for key in path {
        cache.insert(key, outcome);
    }
    outcome
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/released/pty_session_released_ancestry_tests.rs"]
mod tests;
