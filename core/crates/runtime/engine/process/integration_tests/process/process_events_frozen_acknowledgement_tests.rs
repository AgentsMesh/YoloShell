use super::*;
use crate::process::ProcessIdentity;
use crate::process::inventory_test_support::TestProcess;
use std::collections::HashSet;

#[test]
fn frozen_acknowledgement_consumes_only_sources_inside_the_closure() {
    let root = live_identity(std::process::id());
    let foreign = TestProcess::spawn();
    let foreign = live_identity(foreign.identity.pid);
    let mut events = ProcessEvents::new(root);
    events.inject_unresolved_fork(root);
    events.inject_unresolved_fork(foreign);

    assert!(events.acknowledge_frozen_forks(&HashSet::from([root.lineage_id()])));
    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        [foreign.lineage_id()]
    );
    assert!(!events.acknowledge_frozen_forks(&HashSet::from([root.lineage_id()])));
}

#[test]
fn remaining_gone_source_can_use_exact_containment_after_partial_acknowledgement() {
    let root = live_identity(std::process::id());
    let foreign = TestProcess::spawn();
    let foreign = live_identity(foreign.identity.pid);
    let mut events = ProcessEvents::new(root);
    events.inject_unresolved_fork(root);
    events.inject_gone_unresolved_fork(foreign);

    assert!(events.acknowledge_frozen_forks(&HashSet::from([root.lineage_id()])));
    assert!(events.exact_closure_isolated(&[root]));
    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        [foreign.lineage_id()]
    );
}

fn live_identity(pid: u32) -> LiveProcessIdentity {
    let identity = ProcessIdentity::capture(pid).unwrap();
    LiveProcessIdentity::capture(identity).unwrap().unwrap()
}
