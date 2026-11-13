const FINALIZATION_DISPLACED_HOME: &str = "finalization-displaced-home";

#[tokio::test]
async fn lost_namespace_finishes_with_diagnostic_lost_exit() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    let exit_path = fixture.state.paths.exit_record(fixture.terminal_id());
    let displaced = fixture
        ._filesystem
        .scratch_path(FINALIZATION_DISPLACED_HOME);
    std::fs::rename(fixture.state.paths.home(), displaced).unwrap();

    finalize_on_thread(
        Arc::clone(&fixture.state),
        Some(POISON_EXIT_CODE),
        None,
        Duration::ZERO,
    )
    .await;

    let exit = lock_recover(&fixture.state.exit).clone().unwrap();
    assert_eq!(exit.reason, ExitReason::Lost);
    assert!(fixture.state.output_blocked());
    assert!(fixture.state.namespace.prove_ownership_loss().is_some());
    assert!(!exit_path.exists());
    assert_eq!(fixture.state.status().status, TerminalRuntimeStatus::Exited);
}
