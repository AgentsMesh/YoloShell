#[tokio::test]
async fn unavailable_destructive_capability_rejects_termination_without_side_effects() {
    let fixture = Fixture::new();
    let commands = install_test_owner(&fixture);
    let root = fixture.state.pty_root_identity();
    let tombstone = fixture
        .state
        .paths
        .terminate_tombstone(fixture.terminal_id());
    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request(UNAVAILABLE_CAPABILITY_HOLDER),
            false,
        )
        .await
        .unwrap();

    for force in UNAVAILABLE_TERMINATION_ATTEMPTS {
        let error = dispatch_test(
            &fixture.state,
            method::TERMINATE_RUNTIME_EXACT,
            &FencedTerminateRequest {
                terminal_id: fixture.terminal_id(),
                runtime_epoch: fixture.state.runtime_epoch(),
                expected_status: None,
                force,
            },
        )
        .await
        .unwrap_err();
        assert_eq!(
            error.code,
            crate::protocol::error_code::DESTRUCTIVE_SESSION_UNAVAILABLE
        );
        assert!(!error.retryable);
        assert!(matches!(
            commands.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        assert!(!tombstone.try_exists().unwrap());
        assert!(fixture.state.termination_reason().is_none());
        assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Running);
        assert!(root.matches());
    }

    let retained = fixture
        .state
        .get_writer_lease(fixture.controller())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained, lease.lease);
    let input = InputRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability,
        input_id: Uuid::new_v4(),
        data: UNAVAILABLE_CAPABILITY_INPUT.to_vec(),
    };
    assert!(
        dispatch_test(&fixture.state, method::TERMINAL_INPUT, &input)
            .await
            .unwrap()
            .ok
    );
}
