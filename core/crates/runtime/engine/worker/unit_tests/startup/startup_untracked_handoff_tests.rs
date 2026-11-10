use super::*;
use crate::worker::pty_output_progress::{
    ExclusivePtyOutput, OutputProgressState, ReaderBackedPtyOutput,
};
use crate::worker::startup::startup_guard::StartupOutputProgress;
#[path = "../../test_support/unit/startup/startup_untracked_reaper_support.rs"]
mod support;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{TryRecvError, channel};
use std::time::{Duration, Instant};
use support::{ChildObservation, GatedChild};

const CALLER_RETURN_TIMEOUT: Duration = Duration::from_millis(250);
const SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(2);
const WAIT_INTERVAL: Duration = Duration::from_millis(5);
const NO_SIGNALS: usize = 0;

#[test]
fn exclusive_deadline_handoff_returns_without_dropping_custody() {
    let observation = ChildObservation::new();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let guard = StartupChildGuard::with_output(
        Box::new(GatedChild::observed(&observation)),
        ExclusivePtyOutput::observed(Arc::clone(&output_calls)),
    );

    assert_deadline_handoff(guard, observation, output_calls);
}

#[test]
fn reader_backed_deadline_handoff_returns_without_dropping_custody() {
    let observation = ChildObservation::new();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let guard = StartupChildGuard::with_reader_backed_output(
        Box::new(GatedChild::observed(&observation)),
        ReaderBackedPtyOutput::observed(Arc::clone(&output_calls)),
    );

    assert_deadline_handoff(guard, observation, output_calls);
}

#[test]
fn deadline_handoff_never_waits_for_the_spawn_gate() {
    let spawn_gate = crate::process::waitable_child_spawn_guard().unwrap();
    let observation = ChildObservation::new();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let guard = StartupChildGuard::with_output(
        Box::new(GatedChild::observed(&observation)),
        ExclusivePtyOutput::observed(Arc::clone(&output_calls)),
    );

    assert_deadline_handoff(guard, observation, output_calls);
    drop(spawn_gate);
}

#[test]
fn reader_backed_handoff_stays_nonblocking_behind_a_queued_spawner() {
    let spawn_gate = crate::process::waitable_child_spawn_guard().unwrap();
    let waiting_before = spawn_gate.waiting_spawners_for_test();
    let (acquired_sender, acquired_receiver) = channel();
    let contender = std::thread::spawn(move || {
        let _queued = crate::process::waitable_child_spawn_guard().unwrap();
        acquired_sender.send(()).unwrap();
    });
    wait_until(|| spawn_gate.waiting_spawners_for_test() > waiting_before);
    assert_eq!(acquired_receiver.try_recv(), Err(TryRecvError::Empty));

    let observation = ChildObservation::new();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let guard = StartupChildGuard::with_reader_backed_output(
        Box::new(GatedChild::observed(&observation)),
        ReaderBackedPtyOutput::observed(Arc::clone(&output_calls)),
    );
    assert_deadline_handoff(guard, observation, output_calls);
    assert_eq!(acquired_receiver.try_recv(), Err(TryRecvError::Empty));

    drop(spawn_gate);
    acquired_receiver.recv_timeout(SETTLEMENT_TIMEOUT).unwrap();
    contender.join().unwrap();
}

#[test]
fn committed_untracked_loss_releases_completed_output_inline() {
    let observation = ChildObservation::new();
    observation.settled.store(true, Ordering::Release);
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let mut guard = StartupChildGuard::with_output(
        Box::new(GatedChild::observed(&observation)),
        ExclusivePtyOutput::observed(Arc::clone(&output_calls)),
    );
    let gate = guard.unreleased_gate.take().unwrap();
    let proof =
        crate::worker::startup::startup_untracked_proof::UntrackedStartupProducersGone::from_exited_closed_gate(
            gate,
        );
    let StartupOutputProgress::Exclusive(output) = guard.output_progress.as_mut().unwrap() else {
        unreachable!();
    };
    assert_eq!(
        output.mark_untracked_producers_gone(proof).unwrap(),
        OutputProgressState::Complete
    );
    guard.untracked_gone_committed = true;

    assert!(guard.terminate_untracked_at_deadline_for_test().is_err());

    assert!(!guard.requires_cleanup());
    assert!(observation.dropped.load(Ordering::Acquire));
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
}

#[test]
fn committed_untracked_loss_defers_an_active_reader_until_output_settles() {
    let observation = ChildObservation::new();
    let output_calls = Arc::new(AtomicUsize::new(usize::default()));
    let lifecycle = Arc::new(crate::worker::ReaderLifecycle::default());
    assert!(lifecycle.begin_attempt());
    let output = ReaderBackedPtyOutput::observed_with_lifecycle(
        Arc::clone(&output_calls),
        Arc::clone(&lifecycle),
    );
    let mut guard = StartupChildGuard::with_reader_backed_output(
        Box::new(GatedChild::observed(&observation)),
        output,
    );
    observation.settled.store(true, Ordering::Release);
    let gate = guard.unreleased_gate.take().unwrap();
    lifecycle.mark_untracked_producers_gone(
        crate::worker::startup::startup_untracked_proof::UntrackedStartupProducersGone::from_exited_closed_gate(
            gate,
        ),
    );
    guard.untracked_gone_committed = true;

    assert!(guard.terminate_untracked_at_deadline_for_test().is_err());

    assert!(!guard.requires_cleanup());
    assert!(!observation.dropped.load(Ordering::Acquire));
    lifecycle.mark_eof_observed();
    lifecycle.finish_attempt();
    wait_until(|| observation.dropped.load(Ordering::Acquire));
    assert!(output_calls.load(Ordering::Acquire) > usize::default());
    assert!(lifecycle.producers_are_gone());
}

fn assert_deadline_handoff(
    mut guard: StartupChildGuard,
    observation: ChildObservation,
    output_calls: Arc<AtomicUsize>,
) {
    let started = Instant::now();

    assert!(guard.terminate_untracked_at_deadline_for_test().is_err());

    assert!(started.elapsed() < CALLER_RETURN_TIMEOUT);
    assert!(!guard.requires_cleanup());
    output_calls.store(usize::default(), Ordering::Release);
    drop(guard);
    assert!(!observation.dropped.load(Ordering::Acquire));
    wait_until(|| output_calls.load(Ordering::Acquire) > usize::default());
    assert!(!observation.dropped.load(Ordering::Acquire));
    assert_eq!(observation.signals.load(Ordering::Acquire), NO_SIGNALS);
    observation.settled.store(true, Ordering::Release);
    wait_until(|| observation.dropped.load(Ordering::Acquire));
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + SETTLEMENT_TIMEOUT;
    while !predicate() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(WAIT_INTERVAL);
    }
}
