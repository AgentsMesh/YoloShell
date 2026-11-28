use super::*;

#[path = "bootstrap_exec_observer_linux_wait.rs"]
mod wait;
use wait::TraceWait;

#[cfg(any(target_env = "gnu", target_env = "uclibc"))]
pub(crate) type PtraceRequest = libc::c_uint;
#[cfg(not(any(target_env = "gnu", target_env = "uclibc")))]
pub(crate) type PtraceRequest = libc::c_int;

pub(crate) fn trace(
    target: BootstrapExecTarget,
    cancel: Arc<AtomicBool>,
    ready: mpsc::Sender<Result<()>>,
    result: mpsc::Sender<Result<()>>,
) {
    if let Err(error) = seize(target) {
        let _ = ready.send(Err(error));
        return;
    }
    if ready.send(Ok(())).is_err() {
        cancel.store(true, Ordering::Release);
    }
    let _ = result.send(trace_until_exec(target, &cancel));
}

pub(crate) fn seize(target: BootstrapExecTarget) -> Result<()> {
    let thread_id = platform_pid(target.thread_id)?;
    validate_target(target, thread_id)?;
    let options = libc::PTRACE_O_TRACEEXEC | libc::PTRACE_O_TRACEEXIT;
    // SAFETY: the target and option word are validated; PTRACE_SEIZE reads no pointers.
    let result = unsafe {
        libc::ptrace(
            libc::PTRACE_SEIZE as PtraceRequest,
            thread_id,
            std::ptr::null_mut::<libc::c_void>(),
            options as usize as *mut libc::c_void,
        )
    };
    if result == INVALID_SYSTEM_RESULT {
        return Err(std::io::Error::last_os_error())
            .context(BOOTSTRAP_EXEC_OBSERVER_REGISTER_CONTEXT);
    }
    if let Err(error) = validate_target(target, thread_id) {
        let cancel = AtomicBool::new(true);
        if let Err(cleanup_error) = trace_until_exec(target, &cancel) {
            tracing::error!(%cleanup_error, BOOTSTRAP_EXEC_TRACE_STOP_LOG);
        }
        return Err(error);
    }
    Ok(())
}

pub(crate) fn trace_until_exec(target: BootstrapExecTarget, cancel: &AtomicBool) -> Result<()> {
    let mut waiting = TraceWait::new(target)?;
    let mut cancel_deadline = None;
    let mut interrupt_error = None;
    loop {
        let (waited, status) = match waiting.poll() {
            Ok(event) => event,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error).context(BOOTSTRAP_EXEC_OBSERVER_WAIT_CONTEXT),
        };
        if waited == 0 {
            if cancel.load(Ordering::Acquire) && cancel_deadline.is_none() {
                cancel_deadline = Some(Instant::now() + BOOTSTRAP_EXEC_OBSERVER_CANCEL_TIMEOUT);
                interrupt_error = interrupt(waiting.thread_id()).err();
            }
            if cancel_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                return interrupt_error
                    .map_or_else(|| Err(anyhow!(BOOTSTRAP_EXEC_TRACE_CANCEL_CONTEXT)), Err)
                    .context(BOOTSTRAP_EXEC_TRACE_CANCEL_CONTEXT);
            }
            std::thread::sleep(BOOTSTRAP_EXEC_OBSERVER_POLL_INTERVAL);
            continue;
        }
        if libc::WIFEXITED(status) || libc::WIFSIGNALED(status) {
            bail!(BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT);
        }
        ensure!(
            libc::WIFSTOPPED(status),
            BOOTSTRAP_EXEC_OBSERVER_EVENT_CONTEXT
        );
        let event = status >> PTRACE_EVENT_STATUS_SHIFT;
        if libc::WSTOPSIG(status) == libc::SIGTRAP && event == libc::PTRACE_EVENT_EXEC {
            let confirmation = waiting.confirm_exec(waited);
            let detached = detach(waited, NO_SIGNAL);
            confirmation?;
            detached?;
            return Ok(());
        }
        if event == libc::PTRACE_EVENT_EXIT {
            detach(waited, NO_SIGNAL)?;
            bail!(BOOTSTRAP_EXEC_NOT_OBSERVED_CONTEXT);
        }
        if cancel.load(Ordering::Acquire) {
            let signal = detach_signal(event, libc::WSTOPSIG(status));
            detach(waited, signal)?;
            bail!(BOOTSTRAP_EXEC_TRACE_CANCEL_CONTEXT);
        }
        continue_trace(waited, libc::WSTOPSIG(status))?;
    }
}

pub(crate) fn validate_target(target: BootstrapExecTarget, thread_id: libc::pid_t) -> Result<()> {
    ensure!(
        target.process.matches(),
        BOOTSTRAP_EXEC_OBSERVER_IDENTITY_CONTEXT
    );
    ensure!(
        thread_belongs_to_process(target.process.pid, thread_id),
        BOOTSTRAP_EXEC_THREAD_CONTEXT
    );
    Ok(())
}

pub(crate) fn detach_signal(event: libc::c_int, signal: libc::c_int) -> libc::c_int {
    if event == libc::PTRACE_EVENT_STOP && signal == libc::SIGTRAP {
        NO_SIGNAL
    } else {
        signal
    }
}

pub(crate) fn thread_belongs_to_process(process_id: u32, thread_id: libc::pid_t) -> bool {
    let Ok(process_id) = platform_pid(process_id) else {
        return false;
    };
    // SAFETY: tgkill with signal zero checks the exact TGID/TID relationship.
    unsafe { libc::syscall(libc::SYS_tgkill, process_id, thread_id, NO_SIGNAL) == 0 }
}

pub(crate) fn platform_pid(pid: u32) -> Result<libc::pid_t> {
    pid.try_into().context(BOOTSTRAP_EXEC_THREAD_CONTEXT)
}

pub(crate) fn interrupt(thread_id: libc::pid_t) -> Result<()> {
    ptrace_control(PtraceOperation::Interrupt, thread_id).map(|_| ())
}

pub(crate) fn continue_trace(thread_id: libc::pid_t, signal: libc::c_int) -> Result<()> {
    ptrace_control(PtraceOperation::Continue(signal), thread_id).map(|_| ())
}

pub(crate) fn detach(thread_id: libc::pid_t, signal: libc::c_int) -> Result<()> {
    ptrace_control(PtraceOperation::Detach(signal), thread_id).map(|_| ())
}

pub(crate) enum PtraceOperation {
    Interrupt,
    Continue(libc::c_int),
    Detach(libc::c_int),
    ExecThread,
}

pub(crate) fn ptrace_control(
    operation: PtraceOperation,
    thread_id: libc::pid_t,
) -> Result<libc::c_ulong> {
    let mut output: libc::c_ulong = 0;
    let (request, data) = match operation {
        PtraceOperation::Interrupt => (libc::PTRACE_INTERRUPT, std::ptr::null_mut()),
        PtraceOperation::Continue(signal) => {
            (libc::PTRACE_CONT, signal as usize as *mut libc::c_void)
        }
        PtraceOperation::Detach(signal) => {
            (libc::PTRACE_DETACH, signal as usize as *mut libc::c_void)
        }
        PtraceOperation::ExecThread => (
            libc::PTRACE_GETEVENTMSG,
            std::ptr::addr_of_mut!(output).cast(),
        ),
    };
    // SAFETY: the tracee is stopped when required; data is a signal or a writable event word.
    let result = unsafe {
        libc::ptrace(
            request as PtraceRequest,
            thread_id,
            std::ptr::null_mut::<libc::c_void>(),
            data,
        )
    };
    if result == INVALID_SYSTEM_RESULT {
        return Err(std::io::Error::last_os_error()).context(BOOTSTRAP_EXEC_TRACE_CONTROL_CONTEXT);
    }
    Ok(output)
}
