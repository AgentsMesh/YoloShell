const COMPLETION_WAVE5_TIMEOUT: Duration = Duration::from_secs(1);
const COMPLETION_WAVE5_FIRST_FAULT: &str = "first ownership fault";
const COMPLETION_WAVE5_TERMINAL_FAULT: &str = "terminal ownership fault";
const COMPLETION_WAVE5_CHANNEL_CLOSED: &str =
    "PTY fault reaper stopped before publishing its completion";
const COMPLETION_WAVE5_DISPLAY_PANIC: &str = "injected earlier fault Display panic";
const COMPLETION_WAVE5_DROP_PANIC: &str = "injected earlier fault Drop panic";

#[derive(Debug)]
struct PanicEarlierFault(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl std::fmt::Display for PanicEarlierFault {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("{COMPLETION_WAVE5_DISPLAY_PANIC}");
    }
}

impl std::error::Error for PanicEarlierFault {}

impl Drop for PanicEarlierFault {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
        panic!("{COMPLETION_WAVE5_DROP_PANIC}");
    }
}

#[test]
fn completion_publishes_exactly_once() {
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    completion.publish(ReapOutcome::lost_unknown());
    completion.publish(ReapOutcome::waited(None));

    let outcome = receiver.receive_timeout(COMPLETION_WAVE5_TIMEOUT).unwrap();
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::LostUnknown
    );
    assert!(completion.is_published());
}

#[test]
fn completion_wait_is_released_only_after_publication_finishes() {
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    let waiter = completion.clone();
    let rendezvous = std::sync::Arc::new(std::sync::Barrier::new(2));
    let thread_rendezvous = std::sync::Arc::clone(&rendezvous);
    let (finished, publication) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        thread_rendezvous.wait();
        waiter.wait_for_publication();
        finished.send(()).unwrap();
    });
    rendezvous.wait();

    assert!(matches!(
        publication.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    completion.publish(ReapOutcome::waited(None));
    publication.recv_timeout(COMPLETION_WAVE5_TIMEOUT).unwrap();
    thread.join().unwrap();
    assert_eq!(
        receiver
            .receive_timeout(COMPLETION_WAVE5_TIMEOUT)
            .unwrap()
            .evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
}

#[test]
fn completion_contains_dropped_endpoints_and_logs_faults() {
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    drop(receiver);
    let mut outcome = ReapOutcome::lost_unknown();
    outcome.fault = Some(anyhow::anyhow!(COMPLETION_WAVE5_FIRST_FAULT));
    completion.publish(outcome);
    assert!(completion.is_published());

    let (completion, receiver) = FaultReapCompletion::owner_channel();
    drop(completion);
    let outcome = receiver.receive();
    assert!(
        outcome
            .fault
            .unwrap()
            .to_string()
            .contains(COMPLETION_WAVE5_CHANNEL_CLOSED)
    );

    FaultReapCompletion::log().publish(ReapOutcome::lost_unknown());
    let mut failed = ReapOutcome::lost_unknown();
    failed.fault = Some(anyhow::anyhow!(COMPLETION_WAVE5_TERMINAL_FAULT));
    FaultReapCompletion::log().publish(failed);
}

#[test]
fn terminal_fault_retains_the_complete_failure_chain() {
    let mut failure = Some(anyhow::anyhow!(COMPLETION_WAVE5_FIRST_FAULT));
    failure::remember_terminal(
        &mut failure,
        anyhow::anyhow!(COMPLETION_WAVE5_TERMINAL_FAULT),
    );

    let failure = failure.unwrap().to_string();
    assert!(failure.contains(COMPLETION_WAVE5_FIRST_FAULT));
    assert!(failure.contains(COMPLETION_WAVE5_TERMINAL_FAULT));
}

#[test]
fn terminal_fault_contains_earlier_error_display_and_drop_panics() {
    let dropped = std::sync::Arc::new(std::sync::atomic::AtomicBool::default());
    let mut failure = Some(anyhow::Error::new(PanicEarlierFault(
        std::sync::Arc::clone(&dropped),
    )));
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        failure::remember_terminal(
            &mut failure,
            anyhow::anyhow!(COMPLETION_WAVE5_TERMINAL_FAULT),
        );
    }));

    assert!(outcome.is_ok());
    assert!(dropped.load(std::sync::atomic::Ordering::Acquire));
    assert!(
        failure
            .unwrap()
            .to_string()
            .contains(COMPLETION_WAVE5_TERMINAL_FAULT)
    );
}

#[test]
fn target_panic_is_contained_and_releases_publication_waiters_with_diagnostics() {
    let completion = FaultReapCompletion::panicking_for_test();
    let waiter = completion.clone();
    let (finished, publication) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        waiter.wait_for_publication();
        finished.send(()).unwrap();
    });

    completion.publish(ReapOutcome::waited(None));

    publication.recv_timeout(COMPLETION_WAVE5_TIMEOUT).unwrap();
    thread.join().unwrap();
    assert!(completion.is_published());
    assert!(
        completion
            .publication_fault_for_test()
            .unwrap()
            .contains(super::completion::INJECTED_COMPLETION_TARGET_PANIC)
    );
}

#[test]
fn panic_on_drop_payload_cannot_leave_publication_pending() {
    let completion = FaultReapCompletion::drop_panicking_for_test();
    let waiter = completion.clone();
    let (finished, publication) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        waiter.wait_for_publication();
        finished.send(()).unwrap();
    });

    completion.publish(ReapOutcome::waited(None));

    publication.recv_timeout(COMPLETION_WAVE5_TIMEOUT).unwrap();
    thread.join().unwrap();
    assert!(completion.is_published());
    assert_eq!(
        completion.publication_fault_for_test().as_deref(),
        Some(super::completion::COMPLETION_TARGET_PANIC_DETAIL)
    );
}
