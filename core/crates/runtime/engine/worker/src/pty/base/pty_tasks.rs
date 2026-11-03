#[cfg(test)]
pub(crate) fn spawn_waiter(
    child: Box<dyn portable_pty::Child + Send + Sync>,
    state: Arc<WorkerState>,
) -> Result<std::thread::JoinHandle<()>> {
    let identity = child.process_id().map(|pid| ChildIdentity {
        pid,
        start_time: process::process_start_time(pid).ok(),
    });
    let worker = Arc::clone(&state);
    let waiter = std::thread::Builder::new()
        .name(format!(
            "{}-{}",
            PTY_OWNER_THREAD_PREFIX,
            state.terminal_id()
        ))
        .spawn(move || supervise_waiter(child, worker, identity))
        .context("spawn child wait thread")?;
    Ok(waiter)
}

#[cfg(test)]
pub(crate) fn supervise_waiter(
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    state: Arc<WorkerState>,
    identity: Option<ChildIdentity>,
) {
    let status = wait_for_child(child.as_mut(), identity);
    let code = status.as_ref().map(|status| status.exit_code() as i32);
    let signal = status
        .as_ref()
        .and_then(|status| status.signal())
        .and_then(signal_number);
    finalize_waited_child(&state, code, signal);
}

#[cfg(test)]
pub(crate) fn wait_for_child<C>(
    child: &mut C,
    identity: Option<ChildIdentity>,
) -> Option<portable_pty::ExitStatus>
where
    C: portable_pty::Child + ?Sized,
{
    let mut failure_count = 0_u32;
    loop {
        match supervision::catch_thread(supervision::CHILD_WAITER_TASK, || {
            child.wait().context("wait for PTY child")
        }) {
            Ok(status) => return Some(status),
            Err(failure) => {
                if !child_identity_is_live(identity) {
                    let _ = supervision::contain(|| {
                        tracing::warn!(
                            task = failure.task,
                            detail = failure.detail,
                            fallback = CHILD_IDENTITY_LOST_DETAIL,
                            "child waiter lost a terminated child; recording an unknown exit status"
                        );
                    });
                    return None;
                }
                failure_count = failure_count.saturating_add(1);
                let delay = supervision::report_restart(
                    &failure,
                    supervision::RestartPolicy::default(),
                    failure_count,
                );
                std::thread::sleep(delay);
            }
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) struct ChildIdentity {
    pub(crate) pid: u32,
    pub(crate) start_time: Option<u64>,
}

#[cfg(test)]
pub(crate) fn child_identity_is_live(identity: Option<ChildIdentity>) -> bool {
    identity.is_none_or(|identity| match identity.start_time {
        Some(start_time) => process::process_identity_matches(identity.pid, start_time),
        None => process::is_process_alive(identity.pid),
    })
}

pub(crate) fn finalize_waited_child(
    state: &Arc<WorkerState>,
    code: Option<i32>,
    signal: Option<i32>,
) {
    let result = supervision::catch_thread(supervision::CHILD_WAITER_TASK, || {
        finalize_child_exit(state, code, signal, PTY_READER_DRAIN_TIMEOUT);
        Ok(())
    });
    if let Err(failure) = result {
        supervision::report_contained(&failure);
    }
}

pub(crate) fn finalize_lost_unknown_child(state: &Arc<WorkerState>) {
    state.begin_system_termination_blocking(ExitReason::Lost);
    let result = supervision::catch_thread(supervision::CHILD_WAITER_TASK, || {
        finalize_lost_unknown_exit(state, PTY_READER_DRAIN_TIMEOUT);
        Ok(())
    });
    if let Err(failure) = result {
        supervision::report_contained(&failure);
    }
}
