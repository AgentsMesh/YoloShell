use super::*;
use yolo_process_test_support::ProcessIdentityObservation;

const PROCESS_IDENTITY_SURVIVED_DETAIL: &str = "process identity survived cleanup";

pub(crate) async fn wait_for_identity_gone(identity: ProcessIdentity) {
    wait_for_identity_gone_with_budget(identity, PROCESS_EXIT_TIMEOUT).await;
}

pub(crate) async fn wait_for_identity_gone_with_budget(
    identity: ProcessIdentity,
    budget: Duration,
) {
    let deadline = tokio::time::Instant::now() + budget;
    while identity.observe() != ProcessIdentityObservation::Gone
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(PROCESS_POLL_INTERVAL).await;
    }
    assert!(
        identity.observe() == ProcessIdentityObservation::Gone,
        "{PROCESS_IDENTITY_SURVIVED_DETAIL}: {identity:?}"
    );
}
