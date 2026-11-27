use crate::worker::tests::*;
use crate::worker::pty_missing_eof::{
    MissingEofDeadline, OutputAbandonment,
};
use crate::worker::pty_output_progress::OutputProgressState;

#[tokio::test]
async fn abandoned_missing_eof_finalizes_durable_and_runtime_lost_exit() {
    let fixture = Fixture::new();
    let terminal_id = fixture.terminal_id();
    let expired = MissingEofDeadline::with_timeout(Duration::ZERO)
        .observe(OutputProgressState::Quiescent, Instant::now())
        .unwrap()
        .unwrap();

    assert!(matches!(
        fixture.state.reader_lifecycle.settle_missing_eof(expired),
        OutputAbandonment::Abandoned(_)
    ));
    let state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || state.begin_lost_output_abandonment())
        .await
        .unwrap();
    let provisional: ExitRecord = read_json(&fixture.state.paths.exit_record(terminal_id)).unwrap();
    assert_eq!(provisional.reason, ExitReason::Lost);
    assert!(fixture.state.reader_lifecycle.is_cancelled());
    assert!(
        lock_recover(&fixture.state.reader_lifecycle.progress).read_stopped
    );
    assert!(lock_recover(&fixture.state.master).is_some());

    finalize_on_thread(Arc::clone(&fixture.state), None, None, Duration::ZERO).await;

    assert!(lock_recover(&fixture.state.master).is_none());
    let durable: ExitRecord = read_json(&fixture.state.paths.exit_record(terminal_id)).unwrap();
    assert_eq!(durable.reason, ExitReason::Lost);
    let runtime = fixture.state.status();
    assert_eq!(runtime.status, TerminalRuntimeStatus::Exited);
    assert_eq!(runtime.exit.unwrap().reason, ExitReason::Lost);
}
