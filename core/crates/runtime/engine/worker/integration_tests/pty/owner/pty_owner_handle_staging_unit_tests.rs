use super::*;
use crate::worker::pty_owner_handle::PtyOwnerCommandFailure;

const INJECTED_OWNER_SPAWN_FAILURE: &str = "injected PTY owner thread spawn failure";
const UNEXPECTED_OWNER_TASK_RUN: &str = "owner task ran after injected spawn failure";

#[test]
fn owner_thread_spawn_failure_drops_publisher_and_settles() {
    let (settlement, publisher) = new_owner_settlement();
    let observed = Arc::clone(&settlement);

    let result = spawn_owner_thread_with(
        PTY_OWNER_THREAD_PREFIX.to_owned(),
        publisher,
        || unreachable!("{UNEXPECTED_OWNER_TASK_RUN}"),
        |_, _owner| Err(io::Error::other(INJECTED_OWNER_SPAWN_FAILURE)),
    );
    let error = match result {
        Ok(_) => panic!("{UNEXPECTED_OWNER_TASK_RUN}"),
        Err(error) => error,
    };

    assert!(error.to_string().contains(INJECTED_OWNER_SPAWN_FAILURE));
    let failure = observed.get().unwrap().as_ref().unwrap_err();
    assert!(matches!(failure, PtyOwnerCommandFailure::Owner(_)));
    assert!(failure.contains(super::super::settlement::OWNER_SETTLEMENT_ABANDONED_DETAIL));
}
