const SESSION_REMOVAL_DISPLACED_HOME: &str = "session-removal-displaced-home";
const SESSION_REMOVAL_TERMINATION_TIMEOUT: Duration = Duration::from_secs(2);

include!("session_removal_polling_tests.rs");
include!("session_removal_admission_tests.rs");

#[test]
fn ordinary_namespace_io_faults_issue_no_termination_token() {
    let fixture = Fixture::new();
    let home = fixture.state.paths.home().to_path_buf();
    for code in [libc::EIO, libc::EMFILE, libc::EACCES] {
        let verification =
            NamespaceVerification::Indeterminate(std::io::Error::from_raw_os_error(code).into());
        let observation = classify_namespace_observation(home.clone(), verification);

        assert!(into_session_removal(observation).is_err());
        assert_eq!(fixture.state.termination_reason(), None);
        assert!(!fixture.state.reader_lifecycle.is_cancelled());
    }
}

#[tokio::test]
async fn retained_identity_loss_consumes_one_token_at_the_termination_boundary() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let home = fixture.state.paths.home().to_path_buf();
    let displaced = fixture
        ._filesystem
        .scratch_path(SESSION_REMOVAL_DISPLACED_HOME);
    std::fs::rename(&home, displaced).unwrap();
    let observation =
        classify_namespace_observation(home.clone(), fixture.state.namespace.verification());

    let removal = into_session_removal(observation).unwrap().unwrap();
    assert!(matches!(removal, SessionRemoval::NamespaceLost { .. }));
    assert_eq!(removal.path(), home);
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .arm();
    let state = Arc::clone(&fixture.state);
    let terminal_id = fixture.terminal_id();
    let termination = tokio::spawn(async move { removal.terminate(&state, terminal_id).await });
    tokio::time::timeout(
        SESSION_REMOVAL_TERMINATION_TIMEOUT,
        fixture
            .state
            .control_test_hooks
            .terminate_before_delivery
            .wait_until_reached(),
    )
    .await
    .unwrap();
    assert_eq!(fixture.state.termination_reason(), Some(ExitReason::Lost));
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .resume();
    tokio::time::timeout(SESSION_REMOVAL_TERMINATION_TIMEOUT, termination)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
}
