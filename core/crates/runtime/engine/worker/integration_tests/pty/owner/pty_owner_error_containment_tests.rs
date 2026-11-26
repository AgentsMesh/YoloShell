use crate::worker::tests::*;
const OWNER_CONTAINMENT_OUTER_PANIC: &str = "injected PTY owner outer unwind";
const OWNER_CONTAINMENT_DISPLAY_PANIC: &str = "injected PTY owner error Display panic";
const OWNER_CONTAINMENT_SETTLEMENT_DETAIL: &str =
    "PTY owner containment did not publish an authoritative settlement";

#[derive(Debug)]
struct OwnerContainmentError {
    error_dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl std::fmt::Display for OwnerContainmentError {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        panic!("{OWNER_CONTAINMENT_DISPLAY_PANIC}");
    }
}

impl std::error::Error for OwnerContainmentError {}

impl Drop for OwnerContainmentError {
    fn drop(&mut self) {
        self.error_dropped.store(true, Ordering::SeqCst);
        std::panic::panic_any(OwnerContainmentPayload(Arc::clone(&self.payload_dropped)));
    }
}

struct OwnerContainmentObserver {
    error_dropped: Arc<AtomicBool>,
    payload_dropped: Arc<AtomicBool>,
}

impl pty_exit_observer::RootExitObserver for OwnerContainmentObserver {
    fn poll_exited_unreaped(
        &mut self,
        _authority: &process::RootWaitAuthority,
    ) -> Result<Option<process::UnreapedRootExit>> {
        Err(anyhow::Error::new(OwnerContainmentError {
            error_dropped: Arc::clone(&self.error_dropped),
            payload_dropped: Arc::clone(&self.payload_dropped),
        }))
    }
}

struct OwnerRunOnDrop(Option<pty_owner::PtyOwner>);

impl Drop for OwnerRunOnDrop {
    fn drop(&mut self) {
        self.0.take().expect("PTY owner is present").run();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owner_contains_error_and_trace_panics_while_outer_thread_is_unwinding() {
    let fixture = Fixture::new();
    let reader = lock_recover(&fixture.state.master)
        .as_ref()
        .unwrap()
        .try_clone_reader()
        .unwrap();
    spawn_reader(reader, Arc::clone(&fixture.state)).unwrap();
    let ownership = take_fault_ownership(&fixture);
    let (_commands, receiver) = std::sync::mpsc::channel();
    let error_dropped = Arc::new(AtomicBool::default());
    let payload_dropped = Arc::new(AtomicBool::default());
    let trace_events = Arc::new(std::sync::atomic::AtomicUsize::default());
    let trace_payload_dropped = Arc::new(AtomicBool::default());
    let settlement = crate::worker::pty_owner_handle::new_owner_settlement_for_test();
    let owner = ownership.into_owner_for_test(
        Arc::downgrade(&fixture.state),
        Box::new(OwnerContainmentObserver {
            error_dropped: Arc::clone(&error_dropped),
            payload_dropped: Arc::clone(&payload_dropped),
        }),
        receiver,
        Arc::clone(&settlement),
    );
    let subscriber = OwnerContainmentSubscriber {
        events: Arc::clone(&trace_events),
        payload_dropped: Arc::clone(&trace_payload_dropped),
    };

    let outcome = std::thread::spawn(move || {
        let _tracing_test = crate::tracing_test_support::serialize_subscriber_tests();
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tracing::subscriber::with_default(subscriber, || {
                let _owner = OwnerRunOnDrop(Some(owner));
                panic!("{OWNER_CONTAINMENT_OUTER_PANIC}");
            });
        }))
    })
    .join()
    .expect("PTY owner containment thread survives secondary panics");

    assert_eq!(
        owner_containment_panic_message(outcome.unwrap_err().as_ref()),
        Some(OWNER_CONTAINMENT_OUTER_PANIC)
    );
    assert!(trace_events.load(Ordering::SeqCst) > 0);
    assert!(!trace_payload_dropped.load(Ordering::SeqCst));
    let completion = tokio::time::timeout(OWNER_FAULT_TIMEOUT, settlement.wait())
        .await
        .expect(OWNER_CONTAINMENT_SETTLEMENT_DETAIL);
    assert!(matches!(
        completion,
        Err(pty_owner_handle::PtyOwnerCommandFailure::Owner(_))
    ));
    wait_for_runtime_status(&fixture.state, TerminalRuntimeStatus::Exited, OWNER_FAULT_TIMEOUT).await;
    assert!(error_dropped.load(Ordering::SeqCst));
    assert!(!payload_dropped.load(Ordering::SeqCst));
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
}

fn owner_containment_panic_message(payload: &(dyn std::any::Any + Send)) -> Option<&str> {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
}
