use super::*;
use crate::worker::ReaderLifecycle;
use crate::worker::control_test_hooks::BlockingControlHook;
use crate::worker::pty_output_progress::{ExclusivePtyOutput, KernelPtyEofIdentity};
use crate::worker::reader_lifecycle::PendingOutput;
use crate::worker::reader_lifecycle::namespace_loss::NamespaceLossDrainState;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const PENDING_OUTPUT_BYTES: &[u8] = b"pending-before-reader-handoff";
const ACTIVATION_PANIC_DETAIL: &str = "injected reader activation panic";

#[derive(Clone, Copy)]
enum ExclusiveEofKind {
    Raw,
    Sealed,
    Committed,
}

impl ExclusiveEofKind {
    const ALL: [Self; 3] = [Self::Raw, Self::Sealed, Self::Committed];

    fn install(self, output: &mut ExclusivePtyOutput) -> Arc<KernelPtyEofIdentity> {
        match self {
            Self::Raw => output.install_raw_eof_for_activation_test(),
            Self::Sealed => output.install_sealed_eof_for_activation_test(),
            Self::Committed => output.install_committed_eof_for_activation_test(),
        }
    }
}

#[derive(Clone, Copy)]
enum LifecycleConflict {
    Active,
    Pending,
    InFlight,
    Cancelled,
    ReadStopped,
    ProducersGone,
    SupervisorCompleted,
    NamespaceLoss,
    ExistingEof,
}

impl LifecycleConflict {
    const ALL: [Self; 9] = [
        Self::Active,
        Self::Pending,
        Self::InFlight,
        Self::Cancelled,
        Self::ReadStopped,
        Self::ProducersGone,
        Self::SupervisorCompleted,
        Self::NamespaceLoss,
        Self::ExistingEof,
    ];

    fn install(self, lifecycle: &ReaderLifecycle) {
        match self {
            Self::Active => lifecycle.lock_progress().active = true,
            Self::Pending => {
                lifecycle.lock_progress().pending = vec![PendingOutput {
                    data: PENDING_OUTPUT_BYTES.to_vec(),
                    attempt: None,
                }];
            }
            Self::InFlight => lifecycle.lock_progress().output_read_in_flight = true,
            Self::Cancelled => lifecycle.cancelled.store(true, Ordering::Release),
            Self::ReadStopped => lifecycle.mark_read_stopped(),
            Self::ProducersGone => lifecycle.mark_producers_gone_for_test(),
            Self::SupervisorCompleted => lifecycle.mark_supervisor_completed_for_test(),
            Self::NamespaceLoss => {
                let mut progress = lifecycle.lock_progress();
                progress.output_abandoned = true;
                progress.namespace_loss_drain = NamespaceLossDrainState::Active;
            }
            Self::ExistingEof => lifecycle.mark_eof_observed(),
        }
    }
}

#[test]
fn activation_panic_restores_every_exclusive_eof_state_without_poisoning_lifecycle() {
    for kind in ExclusiveEofKind::ALL {
        for previous_wakeup in [false, true] {
            let lifecycle = ReaderLifecycle::default();
            lifecycle
                .output_reader_wakeup
                .store(previous_wakeup, Ordering::Release);
            let mut output = ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default()));
            let expected = kind.install(&mut output);

            let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                lifecycle
                    .adopt_exclusive_kernel_eof_before(&mut output, || {
                        panic!("{ACTIVATION_PANIC_DETAIL}")
                    })
                    .unwrap();
            }));

            assert!(panic.is_err());
            assert!(!lifecycle.progress.is_poisoned());
            assert!(lifecycle.kernel_eof_identity().is_none());
            assert_eq!(
                lifecycle.output_reader_wakeup.load(Ordering::Acquire),
                previous_wakeup
            );
            lifecycle
                .adopt_exclusive_kernel_eof_before(&mut output, || Ok(()))
                .unwrap();
            let restored = lifecycle.kernel_eof_identity().unwrap();
            assert!(Arc::ptr_eq(&expected, &restored));
        }
    }
}

#[test]
fn activation_barrier_keeps_reader_behind_all_exclusive_eof_migrations() {
    for kind in ExclusiveEofKind::ALL {
        assert_reader_starts_after_eof_migration(kind);
    }
}

fn assert_reader_starts_after_eof_migration(kind: ExclusiveEofKind) {
    let (mut pending, receiver) = PendingPtyReader::for_test();
    let lifecycle = pending.lifecycle_for_test();
    let barrier = Arc::new(BlockingControlHook::default());
    barrier.arm();
    pending.install_activation_barrier_for_test(Arc::clone(&barrier));
    let mut output = ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default()));
    let expected = kind.install(&mut output);
    let reader_started = Arc::new(AtomicBool::new(false));
    let reader_started_observer = Arc::clone(&reader_started);
    let lifecycle_observer = Arc::clone(&lifecycle);
    let reader = std::thread::spawn(move || {
        receiver.recv().unwrap();
        reader_started_observer.store(true, Ordering::Release);
        lifecycle_observer.kernel_eof_identity()
    });
    let activation = std::thread::spawn(move || {
        assert!(pending.activate(output).is_ok());
    });

    barrier.wait_until_reached();
    assert!(!reader_started.load(Ordering::Acquire));
    barrier.resume();

    activation.join().unwrap();
    let observed = reader.join().unwrap().unwrap();
    assert!(Arc::ptr_eq(&expected, &observed));
}

#[test]
fn adoption_rejects_every_non_fresh_reader_lifecycle() {
    for conflict in LifecycleConflict::ALL {
        assert_handoff_rejected(conflict);
    }
}

fn assert_handoff_rejected(conflict: LifecycleConflict) {
    let (pending, receiver) = PendingPtyReader::for_test();
    let lifecycle = pending.lifecycle_for_test();
    conflict.install(&lifecycle);
    let mut output = ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default()));
    let expected = output.install_raw_eof_for_activation_test();

    let failure = pending.activate(output).err().unwrap();

    let retained = failure.output.kernel_eof_identity_for_test().unwrap();
    assert!(Arc::ptr_eq(&expected, &retained));
    assert!(matches!(
        receiver.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Disconnected)
    ));
}

#[test]
fn adoption_rejects_an_abandoned_source_output() {
    let (pending, receiver) = PendingPtyReader::for_test();
    let mut output = ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default()));
    let expected = output.install_raw_eof_for_activation_test();
    output.abandon_for_activation_test();

    let failure = pending.activate(output).err().unwrap();

    let retained = failure.output.kernel_eof_identity_for_test().unwrap();
    assert!(Arc::ptr_eq(&expected, &retained));
    assert!(matches!(
        receiver.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Disconnected)
    ));
}
