use super::OutputQuiescenceState;
use crate::worker::ReaderLifecycle;
use crate::worker::pty_output_progress::{
    ExclusiveOutputQuiescenceFence, ExclusivePtyOutput, OutputQueueState, OutputQuiescenceFence,
    ReaderBackedPtyOutput,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const EXPECTED_EXCLUSIVE_PROGRESS_CALLS: usize = 2;
const OUTPUT_AFTER_DRAIN: &[u8] = b"output-after-drain";

#[test]
fn output_fence_requires_post_resume_durable_quiescence() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::new(AtomicUsize::default()),
        Arc::clone(&lifecycle),
    );
    let mut recovery = output.recovery();

    let mut fence: OutputQuiescenceFence = recovery.begin_quiescence_fence().unwrap();
    let stale_read = lifecycle.begin_output_read().unwrap();

    assert!(!lifecycle.output_reader_wakeup.swap(false, Ordering::AcqRel));
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Pending
    );
    assert!(lifecycle.output_reader_wakeup.swap(false, Ordering::AcqRel));
    stale_read.mark_quiescent().unwrap();
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Pending
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Reached
    );
}

#[test]
fn output_returning_after_empty_restarts_the_epoch_fence() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut fence = OutputQuiescenceFence::AwaitingQueue {
        lifecycle: Arc::clone(&lifecycle),
    };

    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Pending
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Pending).unwrap(),
        OutputQuiescenceState::Pending
    );
    let stale_read = lifecycle.begin_output_read().unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Pending
    );
    stale_read.mark_quiescent().unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Pending
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Reached
    );
}

#[test]
fn reader_output_revokes_a_completed_drain_epoch() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let mut fence = OutputQuiescenceFence::AwaitingQueue {
        lifecycle: Arc::clone(&lifecycle),
    };
    fence.observe_queue(OutputQueueState::Empty).unwrap();
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();

    let output_read = lifecycle.begin_output_read().unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Pending
    );
    output_read.stage_for_test(OUTPUT_AFTER_DRAIN).unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Pending
    );
    lifecycle.clear_pending_output_for_test();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Pending
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        fence.observe_queue(OutputQueueState::Empty).unwrap(),
        OutputQuiescenceState::Reached
    );
}

#[test]
fn exclusive_fence_requires_a_second_post_empty_read() {
    let observed = Arc::new(AtomicUsize::default());
    let mut output = ExclusivePtyOutput::observed(Arc::clone(&observed));
    let mut fence: ExclusiveOutputQuiescenceFence = ExclusivePtyOutput::begin_quiescence_fence();

    assert_eq!(
        output.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Pending
    );
    assert_eq!(
        output.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Reached
    );
    assert_eq!(
        observed.load(Ordering::Acquire),
        EXPECTED_EXCLUSIVE_PROGRESS_CALLS
    );
}

#[test]
fn pending_epoch_rearms_an_exhausted_reader_for_a_fresh_observation() {
    let lifecycle = Arc::new(ReaderLifecycle::default());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::new(AtomicUsize::default()),
        Arc::clone(&lifecycle),
    );
    let mut recovery = output.recovery();
    let mut fence = recovery.begin_quiescence_fence().unwrap();
    drop(lifecycle.begin_output_read().unwrap());
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Pending
    );
    drop(lifecycle.begin_output_read().unwrap());
    let confirmed = Arc::new(std::sync::atomic::AtomicBool::default());
    let waiting_lifecycle = Arc::clone(&lifecycle);
    let waiting_confirmation = Arc::clone(&confirmed);
    let waiter = std::thread::spawn(move || {
        waiting_lifecycle.wait_for_child_exit_confirmation(&waiting_confirmation)
    });

    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Pending
    );
    assert_eq!(
        waiter.join().unwrap(),
        crate::worker::reader_lifecycle::ReaderExitConfirmation::OutputProbe
    );
    lifecycle
        .begin_output_read()
        .unwrap()
        .mark_quiescent()
        .unwrap();
    assert_eq!(
        recovery.poll_quiescence_fence(&mut fence).unwrap(),
        OutputQuiescenceState::Reached
    );
}
