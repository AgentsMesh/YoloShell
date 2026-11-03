use crate::process::LiveProcessIdentity;
use std::collections::HashSet;

pub(in crate::process::pty_session_proof) fn exclude_root(
    excluded: &HashSet<LiveProcessIdentity>,
    exact_priorities: &HashSet<LiveProcessIdentity>,
    root_lineage: u64,
    root_exited: bool,
) -> HashSet<LiveProcessIdentity> {
    let mut next = excluded.clone();
    if root_exited {
        next.extend(
            exact_priorities
                .iter()
                .filter(|identity| identity.lineage_id() == root_lineage)
                .cloned(),
        );
    }
    next
}
