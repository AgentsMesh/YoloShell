use super::captured_tree_discovery_test_support::DiscoveryTreeTestFixture;
use super::inventory_snapshot::install_repeating_test_inventory;
use super::{
    DescendantObservation, DestructiveSessionUnavailable, SurvivorPoll, SurvivorReady,
    TerminationUnsupportedProcessTree,
};
use std::time::{Duration, Instant};

const TEST_CAPABILITY: &str = "unsupported-observer-scope";
const SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(5);
const SETTLEMENT_POLL_INTERVAL: Duration = Duration::from_millis(1);
const DISCOVERY_PENDING_WITHOUT_DEBT_DETAIL: &str =
    "process-discovery survivor stayed nonempty after discovery settled";
const DISCOVERY_SETTLEMENT_TIMEOUT_DETAIL: &str =
    "process-discovery survivor did not reach a settled empty observation";
const DISCOVERY_SETTLEMENT_REQUIRED_DETAIL: &str =
    "process-discovery survivor skipped its fail-closed settlement phase";
const FIRST_EMPTY_COMPLETED_DETAIL: &str =
    "process-discovery survivor completed after its first settled empty observation";
const SECOND_EMPTY_PENDING_DETAIL: &str =
    "process-discovery survivor stayed pending after consecutive settled empty observations";

#[test]
fn process_discovery_survivor_retires_through_typed_no_seal_authority() {
    let mut fixture = DiscoveryTreeTestFixture::spawn(TEST_CAPABILITY);
    fixture.exit_root();
    fixture.wait_for_kernel_eof();
    let unavailable = DestructiveSessionUnavailable::new();
    let mut survivor = fixture
        .into_tree()
        .into_termination_unsupported(&unavailable);
    assert!(survivor.retains_pty_proof_for_test());
    survivor.mark_ownership_fault();
    let _empty_inventory = install_repeating_test_inventory(Vec::new());
    survivor = commit_first_settled_empty(survivor);
    let ready = survivor.prepare_poll().unwrap();
    let gone = match survivor.commit_poll(ready) {
        SurvivorPoll::Gone(gone) => gone,
        SurvivorPoll::Pending(_) => panic!("{SECOND_EMPTY_PENDING_DETAIL}"),
    };
    let (reported, ()) = gone.report(|_| Ok(())).unwrap();
    let retired = reported.retire_direct_for_test();

    assert!(retired.session_is_retired());
}

fn commit_first_settled_empty(
    mut survivor: TerminationUnsupportedProcessTree,
) -> TerminationUnsupportedProcessTree {
    let deadline = Instant::now() + SETTLEMENT_TIMEOUT;
    let mut observed_unsettled = false;
    loop {
        let ready = survivor.prepare_poll().unwrap();
        match &ready {
            SurvivorReady::Pending(false) => {
                observed_unsettled = true;
                assert_ne!(
                    survivor.state_snapshot_for_test().descendant_observation,
                    DescendantObservation::Settled,
                    "{DISCOVERY_PENDING_WITHOUT_DEBT_DETAIL}"
                );
            }
            SurvivorReady::Pending(true) => {
                assert!(observed_unsettled, "{DISCOVERY_SETTLEMENT_REQUIRED_DETAIL}");
                return commit_pending(survivor, ready);
            }
            SurvivorReady::Gone => panic!("{FIRST_EMPTY_COMPLETED_DETAIL}"),
        }
        survivor = commit_pending(survivor, ready);
        assert!(
            Instant::now() < deadline,
            "{DISCOVERY_SETTLEMENT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(SETTLEMENT_POLL_INTERVAL);
    }
}

fn commit_pending(
    survivor: TerminationUnsupportedProcessTree,
    ready: SurvivorReady,
) -> TerminationUnsupportedProcessTree {
    match survivor.commit_poll(ready) {
        SurvivorPoll::Pending(survivor) => survivor,
        SurvivorPoll::Gone(_) => panic!("{FIRST_EMPTY_COMPLETED_DETAIL}"),
    }
}
