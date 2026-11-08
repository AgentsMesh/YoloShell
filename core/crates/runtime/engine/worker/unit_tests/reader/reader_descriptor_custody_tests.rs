mod reader_descriptor_custody_tests {
    use super::*;
    use std::os::fd::{AsRawFd, IntoRawFd, RawFd};
    use std::os::unix::net::UnixStream;
    const CUSTODY_ATTEMPT_INCREMENT: usize = 1;
    const CUSTODY_CHANNEL_CAPACITY: usize = 1;
    const CUSTODY_EXPECTED_ATTEMPTS: usize = 2;
    const CUSTODY_FCNTL_FAILURE: libc::c_int = -1;
    const CUSTODY_CLOSE_SUCCESS: libc::c_int = 0;
    const CUSTODY_CLOSE_FAILURE: libc::c_int = -1;
    const CUSTODY_PANIC_DETAIL: &str = "injected reader custody panic";
    const CUSTODY_ERROR_DETAIL: &str = "injected reader custody error";
    const CUSTODY_ATTEMPT_GUARD_DETAIL: &str = "reader attempt should be admitted";
    const CUSTODY_TIMEOUT: Duration = Duration::from_secs(2);
    include!("../../test_support/unit/reader/reader_descriptor_custody_support.rs");
    #[derive(Clone, Copy)]
    enum CustodyAttemptFailure {
        Error,
        Panic,
    }
    #[test]
    fn cancellation_before_first_attempt_closes_initial_reader_before_completion() {
        let fixture = Fixture::new();
        fixture
            .state
            .reader_lifecycle
            .lock_progress()
            .output_abandoned = true;
        fixture.state.reader_lifecycle.cancel();
        let (release, wait_for_release) = mpsc::sync_channel(CUSTODY_CHANNEL_CAPACITY);
        let (reader, dropping, closed) = observed_reader(Some(wait_for_release));
        let state = Arc::clone(&fixture.state);
        let supervisor = std::thread::spawn(move || {
            supervise_reader_with(
                Box::new(reader),
                state,
                u32::default(),
                u32::default(),
                |_, _| unreachable!(),
                |_, _, _| unreachable!(),
            );
        });

        let descriptor = dropping.recv_timeout(CUSTODY_TIMEOUT).unwrap();
        assert_descriptor_open(descriptor);
        assert!(!fixture.state.reader_lifecycle.master_aliases_can_close());
        release.send(()).unwrap();
        let observation = closed.recv_timeout(CUSTODY_TIMEOUT).unwrap();
        assert_eq!(observation.descriptor, descriptor);
        assert_descriptor_closed(observation);
        supervisor.join().unwrap();
        assert!(fixture.state.reader_lifecycle.master_aliases_can_close());
    }

    fn assert_failed_attempt_keeps_custody(failure: CustodyAttemptFailure) {
        let fixture = Fixture::new();
        fixture
            .state
            .reader_lifecycle
            .lock_progress()
            .eof
            .mark_drained();
        let (reader, _dropping, closed) = observed_reader(None);
        let attempts = Arc::new(std::sync::atomic::AtomicUsize::default());
        let attempt_counter = Arc::clone(&attempts);
        let (restart_entered, restart_observed) = mpsc::sync_channel(CUSTODY_CHANNEL_CAPACITY);
        let (release, wait_for_release) = mpsc::sync_channel(CUSTODY_CHANNEL_CAPACITY);
        let state = Arc::clone(&fixture.state);
        let supervisor = std::thread::spawn(move || {
            supervise_reader_with(
                Box::new(reader),
                state,
                u32::default(),
                u32::default(),
                move |_, state| {
                    let attempt =
                        attempt_counter.fetch_add(CUSTODY_ATTEMPT_INCREMENT, Ordering::AcqRel);
                    let _guard = ReaderAttemptGuard::start(&state.reader_lifecycle)
                        .expect(CUSTODY_ATTEMPT_GUARD_DETAIL);
                    if attempt == usize::default() {
                        match failure {
                            CustodyAttemptFailure::Error => anyhow::bail!(CUSTODY_ERROR_DETAIL),
                            CustodyAttemptFailure::Panic => panic!("{CUSTODY_PANIC_DETAIL}"),
                        }
                    }
                    Ok(ReaderLoopOutcome::ChildExited)
                },
                move |_, _, _| {
                    restart_entered.send(()).unwrap();
                    wait_for_release.recv().unwrap();
                },
            );
        });

        let observation = closed.recv_timeout(CUSTODY_TIMEOUT).unwrap();
        restart_observed.recv_timeout(CUSTODY_TIMEOUT).unwrap();
        assert_descriptor_closed(observation);
        assert!(!fixture.state.reader_lifecycle.master_aliases_can_close());
        release.send(()).unwrap();
        supervisor.join().unwrap();
        assert_eq!(attempts.load(Ordering::Acquire), CUSTODY_EXPECTED_ATTEMPTS);
        assert!(fixture.state.reader_lifecycle.master_aliases_can_close());
    }

    #[test]
    fn reader_error_retry_keeps_custody_latched_between_attempts() {
        assert_failed_attempt_keeps_custody(CustodyAttemptFailure::Error);
    }

    #[test]
    fn reader_panic_retry_keeps_custody_latched_between_attempts() {
        assert_failed_attempt_keeps_custody(CustodyAttemptFailure::Panic);
    }

    #[test]
    fn retained_attempt_reader_closes_before_completion() {
        let fixture = Fixture::new();
        fixture
            .state
            .reader_lifecycle
            .lock_progress()
            .output_abandoned = true;
        let (release, wait_for_release) = mpsc::sync_channel(CUSTODY_CHANNEL_CAPACITY);
        let (reader, dropping, closed) = observed_reader(Some(wait_for_release));
        let state = Arc::clone(&fixture.state);
        let supervisor = std::thread::spawn(move || {
            let retained = std::cell::RefCell::new(None);
            supervise_reader_with(
                Box::new(reader),
                state,
                u32::default(),
                u32::default(),
                move |reader, _| {
                    retained.replace(Some(reader));
                    Ok(ReaderLoopOutcome::ChildExited)
                },
                |_, _, _| unreachable!(),
            );
        });
        let descriptor = dropping.recv_timeout(CUSTODY_TIMEOUT).unwrap();
        assert_descriptor_open(descriptor);
        assert!(!fixture.state.reader_lifecycle.master_aliases_can_close());
        release.send(()).unwrap();
        let observation = closed.recv_timeout(CUSTODY_TIMEOUT).unwrap();
        assert_eq!(observation.descriptor, descriptor);
        assert_descriptor_closed(observation);
        supervisor.join().unwrap();
        assert!(fixture.state.reader_lifecycle.master_aliases_can_close());
    }
}
