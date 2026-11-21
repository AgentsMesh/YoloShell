use super::output_block::OutputBlockCause;
use super::{ExitReason, ExitRecord, WorkerState, supervision};
use crate::namespace_lease::NamespaceOwnershipLoss;
use uuid::Uuid;

const EXIT_PERSISTENCE_RETRY_DETAIL: &str = "retrying durable child exit persistence";
const EXIT_PERSISTENCE_NAMESPACE_LOST_DETAIL: &str =
    "child exit cannot be persisted after retained namespace ownership loss";

pub(super) fn persist_exit_until_durable(
    state: &WorkerState,
    terminal_id: Uuid,
    exit: &mut ExitRecord,
) {
    let mut failures = 0_u32;
    loop {
        #[cfg(test)]
        let persisted = if state.control_test_hooks.exit_persist_is_blocked() {
            Err(anyhow::anyhow!(
                super::control_test_hooks::INJECTED_EXIT_PERSIST_FAILURE
            ))
        } else {
            state.persist_json(&state.paths.exit_record(terminal_id), exit)
        };
        #[cfg(not(test))]
        let persisted = state.persist_json(&state.paths.exit_record(terminal_id), exit);
        match persisted {
            Ok(()) => {
                state.clear_output_blocked(OutputBlockCause::ExitPersistence);
                return;
            }
            Err(error) => {
                state.mark_output_blocked(OutputBlockCause::ExitPersistence);
                if let Some(proof) = state.namespace.prove_ownership_loss() {
                    abandon_exit_persistence(state, exit, error, proof);
                    return;
                }
                failures = failures.saturating_add(1);
                let delay = supervision::RestartPolicy::default().delay(failures);
                let detail = supervision::error_detail(error);
                let _ = supervision::contain(|| {
                    tracing::error!(
                        %detail,
                        retry_delay_ms = delay.as_millis(),
                        EXIT_PERSISTENCE_RETRY_DETAIL
                    );
                });
                std::thread::sleep(delay);
            }
        }
    }
}

fn abandon_exit_persistence(
    state: &WorkerState,
    exit: &mut ExitRecord,
    error: anyhow::Error,
    proof: NamespaceOwnershipLoss,
) {
    state.upgrade_finalization_to_lost(proof);
    exit.reason = ExitReason::Lost;
    let detail = supervision::error_detail(error);
    let _ = supervision::contain(|| {
        tracing::error!(%detail, EXIT_PERSISTENCE_NAMESPACE_LOST_DETAIL);
    });
}
