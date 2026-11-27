use super::super::completion_unwind_test_support;
use super::*;
use crate::worker::pty_ownership::ReapEvidence;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::RecvTimeoutError;

const RETAINED_FAILURE: &str = "completion failed before changing retained custody";
const SINGLE_DROP: usize = 1;

struct CompletionPayload(Arc<AtomicUsize>);

impl Drop for CompletionPayload {
    fn drop(&mut self) {
        self.0.fetch_add(SINGLE_DROP, Ordering::SeqCst);
    }
}

fn assert_original_payload(payload: Box<dyn std::any::Any + Send>, drops: &Arc<AtomicUsize>) {
    let original = payload.downcast_ref::<CompletionPayload>().unwrap();
    assert!(Arc::ptr_eq(&original.0, drops));
    assert_eq!(drops.load(Ordering::SeqCst), usize::default());
    drop(payload);
    assert_eq!(drops.load(Ordering::SeqCst), SINGLE_DROP);
}

#[test]
fn finish_outcome_preserves_pending_state_and_original_panic_payload() {
    let mut ownership = passive_ownership();
    let original_custody = std::ptr::from_ref(ownership.custody());
    let payload_drops = Arc::new(AtomicUsize::default());
    let observed_drops = Arc::clone(&payload_drops);
    let _hook = completion_unwind_test_support::install(move |ownership| {
        ownership.remember(Err(anyhow::anyhow!(RETAINED_FAILURE)));
        std::panic::panic_any(CompletionPayload(observed_drops));
    });

    let payload = finish_outcome(&mut ownership).unwrap_err();

    assert_original_payload(payload, &payload_drops);
    assert_eq!(std::ptr::from_ref(ownership.custody()), original_custody);
    assert_eq!(
        ownership.failure.as_ref().unwrap().to_string(),
        RETAINED_FAILURE
    );
    assert!(!ownership.completion().is_published());
    assert!(!ownership.root_gone_without_wait);
    assert!(!finish_outcome(&mut ownership).unwrap());
    ownership.root_gone_without_wait = true;
    assert!(finish_outcome(&mut ownership).unwrap());
    assert!(ownership.completion().is_published());
}

#[test]
fn completion_unwind_leaves_retired_custody_for_the_outer_retry() {
    let root = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let tree = crate::process::CapturedProcessTree::capture(root).unwrap();
    let mut ownership = passive_ownership();
    ownership.custody_mut().processes =
        Some(crate::worker::pty_process_custody::PtyProcessCustody::tracking(tree));
    retire_processes(&mut ownership);
    ownership.root_gone_without_wait = true;
    let before = ownership
        .custody()
        .processes
        .as_ref()
        .unwrap()
        .state_snapshot_for_test();
    let (completion, receiver) = FaultReapCompletion::owner_channel();
    ownership.replace_completion_for_test(completion);
    let payload_drops = Arc::new(AtomicUsize::default());
    let observed_drops = Arc::clone(&payload_drops);
    let _hook = completion_unwind_test_support::install(move |ownership| {
        ownership.remember(Err(anyhow::anyhow!(RETAINED_FAILURE)));
        std::panic::panic_any(CompletionPayload(observed_drops));
    });

    let payload =
        catch_unwind(AssertUnwindSafe(|| reap_without_delay(&mut ownership))).unwrap_err();

    assert_original_payload(payload, &payload_drops);
    assert_eq!(
        ownership
            .custody()
            .processes
            .as_ref()
            .unwrap()
            .state_snapshot_for_test(),
        before
    );
    assert!(ownership.custody().authority.is_none());
    assert!(ownership.lost_wait_custody.is_none());
    assert!(ownership.root_gone_without_wait);
    assert!(!ownership.completion().is_published());
    assert!(ownership.has_unsettled_custody());
    assert_eq!(
        ownership.failure.as_ref().unwrap().to_string(),
        RETAINED_FAILURE
    );

    ownership.reap_retained();

    let outcome = receiver.receive_timeout(Duration::ZERO).unwrap();
    assert_eq!(outcome.evidence, ReapEvidence::LostUnknown);
    assert!(outcome.status.is_none());
    assert_eq!(outcome.fault.unwrap().to_string(), RETAINED_FAILURE);
    assert!(!ownership.has_unsettled_custody());
    assert!(ownership.completion().is_published());
    assert!(matches!(
        receiver.receive_timeout(Duration::ZERO),
        Err(RecvTimeoutError::Timeout)
    ));
}

#[test]
fn already_contained_completion_target_panic_remains_a_completed_publication() {
    let mut ownership = passive_ownership();
    ownership.root_gone_without_wait = true;
    let completion = FaultReapCompletion::panicking_for_test();
    ownership.replace_completion_for_test(completion.clone());

    assert!(finish_outcome(&mut ownership).unwrap());

    assert!(!ownership.has_unsettled_custody());
    assert!(completion.is_published());
    assert!(
        completion.publication_fault_for_test().unwrap().contains(
            crate::worker::pty_fault_reaper::completion::INJECTED_COMPLETION_TARGET_PANIC
        )
    );
}

#[test]
fn completion_unwind_does_not_consume_live_wait_authority() {
    let (root, mut ownership) = waitable_ownership();
    let _cleanup = LiveAuthorityCleanup { root, armed: true };
    let before = ownership.progress();
    let payload_drops = Arc::new(AtomicUsize::default());
    let observed_drops = Arc::clone(&payload_drops);
    let _hook = completion_unwind_test_support::install(move |_| {
        std::panic::panic_any(CompletionPayload(observed_drops));
    });

    let payload = finish_outcome(&mut ownership).unwrap_err();

    assert_original_payload(payload, &payload_drops);
    assert!(ownership.progress() == before);
    assert!(ownership.custody().authority.is_some());
    assert!(ownership.custody().processes.is_some());
    assert!(ownership.custody().root_exit_proof.is_none());
    assert!(ownership.lost_wait_custody.is_none());
    assert!(!ownership.completion().is_published());
    assert!(ownership.has_unsettled_custody());
    assert!(!finish_outcome(&mut ownership).unwrap());
}
