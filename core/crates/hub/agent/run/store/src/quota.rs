//! Orchestration bounds enforced before a run is reserved. Launched runs and
//! Starting runs owned by this process consume a slot; a resumed Starting run
//! from an earlier process does not until it is retried. Depth counts the
//! parent chain regardless of state.

use crate::StoreError;
use std::collections::BTreeSet;

use crate::record::RunRecord;
use yolo_hub_agent_run_protocol::{
    AgentRunAddress, AgentRunState, MAX_ACTIVE_AGENT_RUNS, MAX_AGENT_RUN_CHILDREN,
    MAX_AGENT_RUN_DEPTH,
};
use yolo_hub_terminal_protocol::PublicError;

/// Active runs already consume every slot.
pub(crate) fn full(record: &RunRecord, active: &BTreeSet<AgentRunAddress>) -> bool {
    record
        .runs
        .iter()
        .filter(|run| {
            run.state == AgentRunState::Launched || active.contains(&run.address)
        })
        .count()
        >= MAX_ACTIVE_AGENT_RUNS
}

pub(crate) fn admit(
    record: &RunRecord,
    active: &BTreeSet<AgentRunAddress>,
    parent: Option<AgentRunAddress>,
) -> Result<(), StoreError> {
    if full(record, active) {
        return Err(PublicError::LimitExceeded.into());
    }
    let Some(parent) = parent else {
        return Ok(());
    };
    let children = record
        .runs
        .iter()
        .filter(|run| {
            run.parent == Some(parent)
                && (run.state == AgentRunState::Launched || active.contains(&run.address))
        })
        .count();
    if children >= MAX_AGENT_RUN_CHILDREN || depth(record, parent) >= MAX_AGENT_RUN_DEPTH {
        return Err(PublicError::LimitExceeded.into());
    }
    Ok(())
}

/// Depth of the parent run below its root; a root has depth 0.
fn depth(record: &RunRecord, mut address: AgentRunAddress) -> usize {
    let mut depth = 0;
    while let Some(parent) = record.find(address).and_then(|run| run.parent) {
        depth += 1;
        address = parent;
        if depth > MAX_AGENT_RUN_DEPTH {
            break;
        }
    }
    depth
}
