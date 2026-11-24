use super::*;

pub(super) const EXACT_TERMINATION_STATUS_MISMATCH_DETAIL: &str =
    "worker runtime status does not match the exact termination fence";

pub(super) fn validate_exact_termination_admitted(
    state: &WorkerState,
    _permit: &ControllerMutationPermit,
    request: &FencedTerminateRequest,
) -> std::result::Result<(), ErrorBody> {
    state.validate_runtime(request.terminal_id, request.runtime_epoch)?;
    let observed = state.status().status;
    if request
        .expected_status
        .is_some_and(|expected| !expected.accepts_exact_termination_observation(observed))
    {
        return Err(ErrorBody::conflict(
            EXACT_TERMINATION_STATUS_MISMATCH_DETAIL,
        ));
    }
    Ok(())
}

pub(super) async fn terminate_runtime_admitted(
    state: &Arc<WorkerState>,
    controller: AuthenticatedController,
    permit: ControllerMutationPermit,
    terminal_id: Uuid,
    force: bool,
) -> std::result::Result<(), ErrorBody> {
    let live = lock_recover(&state.exit).is_none();
    if live {
        preflight_pty_termination(state, &permit).map_err(termination_process_error)?;
    }
    let tombstone = TerminationTombstone::user_request(terminal_id, state.runtime_epoch(), force);
    state
        .persist_json(&state.paths.terminate_tombstone(terminal_id), &tombstone)
        .map_err(internal_error)?;
    // Once durable, the owner command must be queued before cancellation is observable.
    state.begin_controller_termination(&permit, ExitReason::UserTerminate);
    let submitted = live
        .then(|| submit_pty_termination(state, &permit, force))
        .transpose()
        .map_err(termination_process_error)?;
    drop(permit);
    #[cfg(feature = "termination-test-hooks")]
    termination_test_hook::pause_after_tombstone(terminal_id)
        .await
        .map_err(internal_error)?;
    #[cfg(test)]
    state
        .control_test_hooks
        .termination_after_tombstone
        .pause_if_armed()
        .await;
    let exited = lock_recover(&state.exit).is_some();
    match (submitted, exited) {
        (Some(submitted), false) => submitted
            .wait(state, force)
            .await
            .map_err(termination_process_error)?,
        (_, true) => state.schedule_controller_shutdown(controller),
        (None, false) => return Err(internal_error(PTY_TERMINATION_SUBMISSION_MISSING_DETAIL)),
    }
    Ok(())
}
