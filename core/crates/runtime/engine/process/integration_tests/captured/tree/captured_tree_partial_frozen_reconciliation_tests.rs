use super::exact_gate_fixture::ExactGateFixture;
use super::incremental_reconciliation_debt_tests::install_isolated_gone_fork;
use super::{FreezeClosureCandidate, exact_stopped_candidate};
use crate::process::captured_tree_macos_wait::{POLL_INTERVAL, TEST_TIMEOUT};
use std::time::Instant;

const TEST_CAPABILITY: &str = "partial-frozen-reconciliation";
const RECONCILIATION_TIMEOUT_DETAIL: &str =
    "fresh frozen-source coverage did not settle independently";

#[test]
fn isolated_foreign_debt_does_not_retain_reconciled_frozen_source_debt() {
    let mut fixture = ExactGateFixture::spawn(TEST_CAPABILITY, false);
    let mut tree = fixture.take_tree();
    let targets = tree.captured_targets();
    let holder = targets[usize::default()].0;
    let foreign = install_isolated_gone_fork(&mut tree, holder);
    tree.events.inject_unresolved_fork(holder);

    assert!(tree.acknowledge_frozen_forks(&[holder]));
    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [foreign.lineage_id()]
    );
    assert!(tree.frozen_fork_reconciliation_required);

    let deadline = Instant::now() + TEST_TIMEOUT;
    while tree.frozen_fork_reconciliation_required {
        tree.refresh().unwrap();
        assert!(Instant::now() < deadline, "{RECONCILIATION_TIMEOUT_DETAIL}");
        std::thread::sleep(POLL_INTERVAL);
    }

    assert!(tree.events.exact_closure_isolated(&[holder]));
    assert!(matches!(
        exact_stopped_candidate(&mut tree).unwrap(),
        Some(FreezeClosureCandidate::Exact(closure))
            | Some(FreezeClosureCandidate::IncrementalExact(closure))
            if closure == [holder]
    ));
    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [foreign.lineage_id()]
    );

    tree.events.inject_unresolved_fork(holder);
    assert!(!tree.events.inventory_publication_is_current());
    assert!(!tree.events.exact_closure_isolated(&[holder]));
    assert!(exact_stopped_candidate(&mut tree).unwrap().is_none());
}
