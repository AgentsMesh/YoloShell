use crate::worker::tests::*;
const CLOSED_RELEASE_COMMAND_DETAIL: &str =
    "closed Host release command channel did not remain retryable";
const DROPPED_RELEASE_COMPLETION_DETAIL: &str =
    "dropped Host release completion did not remain retryable";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn closed_release_command_channel_waits_for_successor_authority() {
    let fixture = Fixture::new();
    let identity = active_custody_identity(&fixture);
    let (handle, commands) = secondary_custody_release::channel(identity);
    drop(commands);
    fixture
        .state
        .pty_alias_custody
        .install_release_handle(handle);

    let (poll, _recovery) = poll_alias_release(observed_release_recovery(&fixture)).await;
    assert!(
        matches!(
            poll.unwrap(),
            pty_output_progress::MasterAliasReleasePoll::Pending
        ),
        "{CLOSED_RELEASE_COMMAND_DETAIL}"
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .released_successor_adoptable()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dropped_release_completion_waits_for_successor_authority() {
    let fixture = Fixture::new();
    let identity = active_custody_identity(&fixture);
    let (handle, mut commands) = secondary_custody_release::channel(identity);
    fixture
        .state
        .pty_alias_custody
        .install_release_handle(handle);

    let (first, recovery) = poll_alias_release(observed_release_recovery(&fixture)).await;
    assert!(matches!(
        first.unwrap(),
        pty_output_progress::MasterAliasReleasePoll::Pending
    ));
    drop(commands.recv().await.unwrap());

    let (second, _recovery) = poll_alias_release(recovery).await;
    assert!(
        matches!(
            second.unwrap(),
            pty_output_progress::MasterAliasReleasePoll::Pending
        ),
        "{DROPPED_RELEASE_COMPLETION_DETAIL}"
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .released_successor_adoptable()
    );
}

fn active_custody_identity(
    fixture: &Fixture,
) -> secondary_custody_authority::ReadyWorkerCustodyIdentity {
    fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap()
}

fn observed_release_recovery(fixture: &Fixture) -> pty_output_progress::PtyOutputRecovery {
    fixture.state.reader_lifecycle.mark_drained();
    let output = pty_output_progress::ReaderBackedPtyOutput::observed_with_release_custody(
        Arc::new(std::sync::atomic::AtomicUsize::new(usize::default())),
        Arc::clone(&fixture.state.reader_lifecycle),
        Arc::clone(&fixture.state.pty_alias_custody),
        runtime_release_authority(&lock_recover(&fixture.state.record)),
    );
    output.recovery()
}
