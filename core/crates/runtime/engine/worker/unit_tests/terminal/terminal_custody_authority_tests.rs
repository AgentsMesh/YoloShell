const TERMINAL_CUSTODY_EXIT_CODE: i32 = 0;
const TERMINAL_CUSTODY_OUTPUT_SEQUENCE: u64 = 0;

#[tokio::test]
async fn durable_terminal_runtime_accepts_only_its_current_controller_generation() {
    let fixture = Fixture::new();
    let current = fixture.controller();
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    fixture.state.mark_custody_unavailable(identity).await;
    assert!(fixture.state.validate_controller(current).is_err());

    *lock_recover(&fixture.state.exit) = Some(ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(TERMINAL_CUSTODY_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: TERMINAL_CUSTODY_OUTPUT_SEQUENCE,
    });

    assert!(fixture.state.validate_controller(current).is_ok());
    assert!(
        fixture
            .state
            .validate_controller(AuthenticatedController::new(CUSTODY_SUCCESSOR_GENERATION))
            .is_err()
    );
}
