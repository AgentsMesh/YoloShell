use super::{Admission, transition};
use crate::pty_custody::CustodyDurableState;
use crate::worker::secondary_custody_persistence;
use std::time::{Duration, Instant};

#[path = "../../test_support/integration/secondary_custody/secondary_custody_guardian_replacement_process_support.rs"]
mod support;
use support::{DetachedGuardian, Fixture};

#[path = "secondary_custody_guardian_fence_process_tests.rs"]
mod fence_process_tests;
#[path = "secondary_custody_guardian_host_b_process_tests.rs"]
mod host_b_process_tests;

const RETIREMENT_TIMEOUT: Duration = Duration::from_secs(8);
const RETIREMENT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const PREPARED_HOLD_OBSERVATION: Duration = Duration::from_millis(100);
const RETIREMENT_TIMEOUT_DETAIL: &str = "successor guardian retirement timed out";

#[tokio::test]
async fn fixture_drop_terminates_a_registered_guardian_without_a_caller_handle() {
    let (identity, home, namespace) = {
        let fixture = Fixture::new();
        let guardian = spawn(&fixture).await.binding().process();
        let identity = crate::process::ProcessIdentity {
            pid: guardian.pid,
            start_time: guardian.start_time,
        };
        assert_eq!(
            identity.observe(),
            crate::process::ProcessIdentityObservation::Matching
        );
        assert!(crate::custody_guardian::test_custody_guardian_is_registered(identity));
        (
            identity,
            fixture.paths.home().to_path_buf(),
            fixture.paths.socket_dir().unwrap(),
        )
    };

    assert_eq!(
        identity.observe(),
        crate::process::ProcessIdentityObservation::Gone
    );
    assert!(!crate::custody_guardian::test_custody_guardian_is_registered(identity));
    assert!(!home.exists());
    assert!(!namespace.exists());
}

#[tokio::test]
async fn real_successor_transfers_custody_arms_and_retires_without_process_leaks() {
    let mut fixture = Fixture::new();
    let preparation = spawn(&fixture).await;
    let mut guardian = DetachedGuardian::capture(preparation);

    let (prepared, prepared_identity) = fixture.current();
    assert_eq!(prepared_identity, preparation.custody_file_identity());
    let CustodyDurableState::PreparedGuardian {
        predecessor,
        candidate,
    } = &prepared.state
    else {
        panic!("successor guardian did not install PreparedGuardian");
    };
    assert_eq!(predecessor.as_ref(), &fixture.predecessor.record().state);
    assert!(preparation.binding().matches_lease_identity(candidate));

    let (expected, armed) = transition::build(
        Admission::Successor,
        fixture.predecessor.record(),
        &fixture.request,
        &fixture.offer,
        Some(&prepared),
    )
    .unwrap();
    assert_eq!(prepared, expected);
    secondary_custody_persistence::persist_transition(
        &fixture.namespace,
        &fixture.paths,
        &fixture.runtime,
        fixture.runtime_identity,
        &expected,
        preparation.custody_file_identity(),
        &armed,
    )
    .unwrap();
    assert_eq!(fixture.current().0, armed);

    fixture.terminate_host();
    fixture.terminate_worker();
    fixture.wait_for_guardian_to_terminate_child();
    wait_for_exact_retirement(&fixture, &armed);
    assert_promoted(&fixture);
    guardian.wait_for_natural_exit();
}

#[tokio::test]
async fn dead_host_before_worker_arm_converges_through_the_replacement_guardian() {
    let mut fixture = Fixture::new();
    let preparation = spawn(&fixture).await;
    let mut guardian = DetachedGuardian::capture(preparation);
    let prepared = fixture.current();
    assert_eq!(prepared.1, preparation.custody_file_identity());

    fixture.terminate_host();
    std::thread::sleep(PREPARED_HOLD_OBSERVATION);
    assert_eq!(fixture.current(), prepared);

    let armed = prepared
        .0
        .arm_exact_prepared_guardian(preparation.binding())
        .unwrap();
    fixture.terminate_worker();
    fixture.wait_for_guardian_to_terminate_child();
    wait_for_exact_retirement(&fixture, &armed);
    assert_promoted(&fixture);
    guardian.wait_for_natural_exit();
}

async fn spawn(fixture: &Fixture) -> crate::pty_custody::CustodyGuardianPreparation {
    crate::custody_guardian::spawn_successor(
        &fixture.executable(),
        &fixture.paths,
        fixture.runtime.terminal_id,
        &fixture.request,
        &fixture.offer,
        fixture.runtime_identity,
        fixture.predecessor.clone(),
        &fixture.descriptors(),
    )
    .await
    .unwrap()
}

fn assert_promoted(fixture: &Fixture) {
    assert_eq!(
        fixture.current_runtime().controller_generation,
        fixture.request.controller_generation
    );
}

fn wait_for_exact_retirement(fixture: &Fixture, armed: &crate::pty_custody::CustodyDurableRecord) {
    let expected = armed.armed_lease().unwrap();
    let deadline = Instant::now() + RETIREMENT_TIMEOUT;
    loop {
        let path = fixture.paths.pty_custody(fixture.runtime.terminal_id);
        let observation = fixture
            .namespace
            .read_json_with_identity::<crate::pty_custody::CustodyDurableRecord>(&path);
        if observation.as_ref().is_ok_and(|(current, _)| {
            current.retired_lease() == Some(expected) && current.retirement_evidence().is_some()
        }) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "{RETIREMENT_TIMEOUT_DETAIL}: {observation:?}"
        );
        std::thread::sleep(RETIREMENT_POLL_INTERVAL);
    }
}
