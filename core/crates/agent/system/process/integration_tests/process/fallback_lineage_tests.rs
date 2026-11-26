use super::super::fallback_observation::{ProcessIdentityApi, ProcessRow};
use super::*;
use std::collections::HashMap;

const ANCESTOR_PID: u32 = 71;
const BACKGROUND_PROCESS_GROUP: i32 = 83;
const DETACHED_PID: u32 = 79;
const LEAF_PID: u32 = 73;
const NO_FOREGROUND_GROUP: i32 = -1;
const ROOT_PID: u32 = 67;
const REAPED_PARENT_PID: u32 = 1;
const IDENTITY_CHANGE: u64 = 1;

struct IdentityMap(HashMap<u32, ProcessIdentity>);

impl ProcessIdentityApi for IdentityMap {
    fn identity(&self, pid: u32) -> Option<ProcessIdentity> {
        self.0.get(&pid).cloned()
    }
}

#[test]
fn captured_lineage_requires_a_fixed_root_and_every_parent_edge() {
    let root = at(ROOT_PID, REAPED_PARENT_PID);
    let ancestor = at(ANCESTOR_PID, ROOT_PID);
    let leaf = at(LEAF_PID, ANCESTOR_PID);
    let rows = [row(&root), row(&ancestor), row(&leaf)];
    let index = identity_index(&rows).unwrap();
    let lineage = captured_lineage(&index, LEAF_PID, &root).unwrap();
    assert_eq!(lineage, [leaf, ancestor, root.clone()]);

    let mut missing = index.clone();
    missing.remove(&ANCESTOR_PID);
    assert!(captured_lineage(&missing, LEAF_PID, &root).is_none());
    let mut cycle = index;
    cycle.get_mut(&ANCESTOR_PID).unwrap().parent_pid = LEAF_PID;
    assert!(captured_lineage(&cycle, LEAF_PID, &root).is_none());
}

#[test]
fn reparented_or_reused_ancestor_rejects_a_stable_leaf() {
    let root = at(ROOT_PID, REAPED_PARENT_PID);
    let ancestor = at(ANCESTOR_PID, ROOT_PID);
    let leaf = at(LEAF_PID, ANCESTOR_PID);
    let expected = vec![leaf.clone(), ancestor.clone(), root.clone()];
    assert!(lineage_is_stable(&map(&expected), &expected));

    let mut reparented = ancestor.clone();
    reparented.parent_pid = REAPED_PARENT_PID;
    assert!(!lineage_is_stable(
        &map(&[leaf.clone(), reparented, root.clone()]),
        &expected,
    ));
    let mut reused_root = root.clone();
    reused_root.start_time += IDENTITY_CHANGE;
    assert!(!lineage_is_stable(
        &map(&[leaf, ancestor, reused_root]),
        &expected,
    ));
}

#[test]
fn foreground_candidates_exclude_shell_background_and_detached_rows() {
    let root = at(ROOT_PID, REAPED_PARENT_PID);
    let foreground = row(&at(LEAF_PID, ROOT_PID));
    let mut background = row(&at(ANCESTOR_PID, ROOT_PID));
    background.process_group = BACKGROUND_PROCESS_GROUP;
    let mut detached = row(&at(DETACHED_PID, ROOT_PID));
    detached.foreground_group = NO_FOREGROUND_GROUP;

    assert!(is_foreground_candidate(&foreground, &root));
    assert!(!is_foreground_candidate(&row(&root), &root));
    assert!(!is_foreground_candidate(&background, &root));
    assert!(!is_foreground_candidate(&detached, &root));

    let mut shell_without_foreground = root.clone();
    shell_without_foreground.foreground_group = NO_FOREGROUND_GROUP;
    assert!(!is_foreground_candidate(
        &foreground,
        &shell_without_foreground,
    ));
}

fn at(pid: u32, parent_pid: u32) -> ProcessIdentity {
    let mut identity = super::super::fallback_observation::test_support::identity();
    identity.pid = pid;
    identity.parent_pid = parent_pid;
    identity.process_inode = u64::from(pid);
    identity
}

fn row(identity: &ProcessIdentity) -> ProcessRow {
    ProcessRow {
        pid: identity.pid,
        parent_pid: identity.parent_pid,
        process_group: identity.process_group,
        foreground_group: identity.foreground_group,
        executable: identity.executable.clone(),
        inventory_identity: Some(identity.clone()),
    }
}

fn map(identities: &[ProcessIdentity]) -> IdentityMap {
    IdentityMap(
        identities
            .iter()
            .map(|identity| (identity.pid, identity.clone()))
            .collect(),
    )
}
