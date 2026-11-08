use super::*;

use self::finalization_persistence::persist_exit_until_durable;
pub(super) use self::finalization_reader::{FinalizationSchedule, await_reader_drain, drain_reader};

#[derive(Clone, Copy)]
pub(super) enum ChildExitEvidence {
    Waited,
    LostUnknown,
}

pub(super) fn finalize_child_exit(
    state: &Arc<WorkerState>,
    code: Option<i32>,
    signal: Option<i32>,
    drain_timeout: Duration,
) {
    finalize_child_exit_scheduled(
        state,
        code,
        signal,
        FinalizationSchedule::production(drain_timeout),
    );
}

pub(super) fn finalize_lost_unknown_exit(state: &Arc<WorkerState>, drain_timeout: Duration) {
    finalize_child_exit_with_evidence(
        state,
        None,
        None,
        FinalizationSchedule::production(drain_timeout),
        ChildExitEvidence::LostUnknown,
    );
}

pub(super) fn finalize_child_exit_scheduled(
    state: &Arc<WorkerState>,
    code: Option<i32>,
    signal: Option<i32>,
    schedule: FinalizationSchedule,
) {
    finalize_child_exit_with_evidence(state, code, signal, schedule, ChildExitEvidence::Waited);
}

pub(super) fn finalize_child_exit_with_evidence(
    state: &Arc<WorkerState>,
    code: Option<i32>,
    signal: Option<i32>,
    schedule: FinalizationSchedule,
    evidence: ChildExitEvidence,
) {
    if state
        .exit_finalization_started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }
    let permit = state.admit_system_mutation_blocking();
    match evidence {
        ChildExitEvidence::Waited => {
            state.publish_direct_child_waited();
        }
        ChildExitEvidence::LostUnknown => state.reader_lifecycle.cancel(),
    }
    contain_finalization_phase(|| {
        let mut writer = lock_recover(&state.writer);
        drop(writer.take());
        Ok(())
    });
    contain_finalization_phase(|| {
        state.revoke_writer_lease_admitted(&permit, LeaseReason::SessionEnded);
        Ok(())
    });
    drop(permit);
    await_reader_drain(state, schedule);
    let permit = state.admit_system_mutation_blocking();
    contain_finalization_phase(|| {
        lock_recover(&state.master).take();
        Ok(())
    });
    drop(permit);
    let record = lock_recover(&state.record);
    let runtime_epoch = record.runtime_epoch;
    let terminal_id = record.terminal_id;
    drop(record);
    let mut exit = ExitRecord {
        runtime_epoch,
        code,
        signal,
        reason: state.termination_reason().unwrap_or(ExitReason::Natural),
        observed_at_ms: now_ms(),
        last_output_sequence: lock_recover(&state.journal).last_sequence(),
    };
    persist_exit_until_durable(state, terminal_id, &mut exit);
    *lock_recover(&state.exit) = Some(exit.clone());
    contain_finalization_phase(|| state.current_snapshot_internal().map(|_| ()));
    let _ = state.events.send(WorkerEvent::Exited(exit));
    if state.has_termination_intent() {
        std::thread::sleep(NATURAL_EXIT_SHUTDOWN_DELAY);
        #[cfg(test)]
        state
            .control_test_hooks
            .finalizer_before_shutdown
            .pause_if_armed();
        state.notify_termination_shutdown_blocking();
    } else {
        state.schedule_natural_exit_shutdown();
    }
}

pub(super) async fn wait_for_reader_shutdown(state: Arc<WorkerState>) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        while drain_reader(&state, PTY_READER_DRAIN_TIMEOUT) {
            let _ = supervision::contain(|| {
                tracing::warn!(
                    terminal_id = %state.terminal_id(),
                    "waiting for retained PTY output delivery after namespace loss"
                );
            });
        }
    })
    .await
    .context("join PTY reader shutdown wait")
}

pub(super) fn contain_finalization_phase(operation: impl FnOnce() -> Result<()>) {
    if let Err(failure) = supervision::catch_thread(supervision::CHILD_WAITER_TASK, operation) {
        supervision::report_contained(&failure);
    }
}

pub(super) fn lock_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        let _ = supervision::contain(|| {
            tracing::error!("recovering poisoned worker shared mutex");
        });
        poisoned.into_inner()
    })
}
