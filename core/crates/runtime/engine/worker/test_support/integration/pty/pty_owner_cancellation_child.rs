const OWNER_CANCELLATION_SETUP_WAIT_COUNT: u32 = 3;
const OWNER_CANCELLATION_SETTLEMENT_WAIT_COUNT: u32 = 4;
const OWNER_CANCELLATION_JOIN_WAIT_COUNT: u32 = 2;
const OWNER_CANCELLATION_CHILD_WATCHDOG_TIMEOUT: Duration = OWNER_CANCELLATION_SETUP_TIMEOUT
    .saturating_mul(OWNER_CANCELLATION_SETUP_WAIT_COUNT)
    .saturating_add(OWNER_CANCELLATION_REQUEST_TIMEOUT)
    .saturating_add(OWNER_CANCELLATION_HANDLER_ABORT_TIMEOUT)
    .saturating_add(OWNER_CANCELLATION_ROOT_EXIT_TIMEOUT)
    .saturating_add(
        OWNER_CANCELLATION_SETTLEMENT_TIMEOUT
            .saturating_mul(OWNER_CANCELLATION_SETTLEMENT_WAIT_COUNT),
    )
    .saturating_add(OWNER_THREAD_JOIN_TIMEOUT.saturating_mul(OWNER_CANCELLATION_JOIN_WAIT_COUNT))
    .saturating_add(OWNER_CANCELLATION_RUNTIME_TIMEOUT);
const OWNER_CANCELLATION_CHILD_REAP_TIMEOUT: Duration = Duration::from_secs(5);
const OWNER_CANCELLATION_PRIVATE_PROCESS_GROUP: i32 = 0;
const OWNER_CANCELLATION_SYSTEM_CALL_FAILURE: libc::pid_t = -1;
const OWNER_CANCELLATION_CHILD_TIMEOUT_DETAIL: &str =
    "isolated owner cancellation child exceeded its watchdog";
const OWNER_CANCELLATION_CHILD_REAP_DETAIL: &str =
    "isolated owner cancellation child was not reaped after SIGKILL";
const OWNER_CANCELLATION_CHILD_ALIAS_DETAIL: &str =
    "isolated cancellation child aliases its parent";
const OWNER_CANCELLATION_CHILD_IDENTITY_DETAIL: &str =
    "isolated cancellation child identity is unavailable";
const OWNER_CANCELLATION_GROUP_DETAIL: &str =
    "isolated cancellation child did not enter its private process group";
const OWNER_CANCELLATION_GROUP_UNKNOWN_DETAIL: &str =
    "isolated cancellation child group identity is unknown";

struct OwnerCancellationChild {
    child: Option<process::DetachedChildCustody>,
    identity: Option<process::ProcessIdentity>,
    containment: Option<process::CapturedProcessTree>,
}

impl OwnerCancellationChild {
    fn spawn(mut command: std::process::Command) -> Result<Self> {
        use std::os::unix::process::CommandExt as _;
        command.process_group(OWNER_CANCELLATION_PRIVATE_PROCESS_GROUP);
        let spawn_guard = process::waitable_child_spawn_guard()?;
        let child = command.spawn()?;
        drop(spawn_guard);
        let child = process::DetachedChildCustody::capture(child)
            .unwrap_or_else(process::DetachedChildCustodyError::into_custody);
        let identity = child.identity();
        let mut guarded = Self {
            child: Some(child),
            identity,
            containment: None,
        };
        let identity = guarded
            .identity
            .context(OWNER_CANCELLATION_CHILD_IDENTITY_DETAIL)?;
        anyhow::ensure!(
            identity.pid != std::process::id(),
            OWNER_CANCELLATION_CHILD_ALIAS_DETAIL
        );
        verify_owner_cancellation_private_group(identity)?;
        let containment = process::CapturedProcessTree::capture(identity)?;
        guarded.containment = Some(containment);
        Ok(guarded)
    }

    fn wait_for_exit(&mut self) -> Result<std::process::ExitStatus> {
        let status = self.poll_until(
            OWNER_CANCELLATION_CHILD_WATCHDOG_TIMEOUT,
            OWNER_CANCELLATION_CHILD_TIMEOUT_DETAIL,
        )?;
        if !status.success() {
            if let Some(containment) = self.containment.as_mut() {
                let _ = containment.terminate(libc::SIGKILL);
            }
        }
        Ok(status)
    }

    fn terminate_and_reap(&mut self) -> Result<std::process::ExitStatus> {
        let mut failure = self.terminate_private_group().err();
        if let Some(containment) = self.containment.as_mut()
            && let Err(error) = containment.terminate(libc::SIGKILL)
        {
            failure.get_or_insert(error);
        }
        if let Some(child) = self.child.as_mut()
            && !matches!(child.request_force_termination(), Ok(true))
        {
            let _ = child.request_direct_termination();
        }
        let status = self.poll_until(
            OWNER_CANCELLATION_CHILD_REAP_TIMEOUT,
            OWNER_CANCELLATION_CHILD_REAP_DETAIL,
        )?;
        failure.map_or(Ok(status), Err)
    }

    fn terminate_private_group(&self) -> Result<()> {
        let Some(identity) = self.identity else {
            return Ok(());
        };
        match identity.observe() {
            process::ProcessIdentityObservation::Matching => {
                let group_id = i32::try_from(identity.pid)?;
                let group =
                    process::PtyProcessScope::capture(identity)?.foreground_group(group_id)?;
                process::signal_process_group(group, libc::SIGKILL)?;
                Ok(())
            }
            process::ProcessIdentityObservation::Gone => Ok(()),
            process::ProcessIdentityObservation::Unknown => {
                anyhow::bail!(OWNER_CANCELLATION_GROUP_UNKNOWN_DETAIL)
            }
        }
    }

    fn poll_until(
        &mut self,
        timeout: Duration,
        detail: &'static str,
    ) -> Result<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        let mut last_error = None;
        loop {
            if let Some(containment) = self.containment.as_mut() {
                let _ = containment.refresh();
            }
            match self.child.as_mut().unwrap().try_wait() {
                Ok(Some(status)) => {
                    self.child.take();
                    return Ok(status);
                }
                Ok(None) => {}
                Err(error) => last_error = Some(error),
            }
            if Instant::now() >= deadline {
                if let Some(error) = last_error {
                    return Err(error).context(detail);
                }
                anyhow::bail!(detail);
            }
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }
}

impl Drop for OwnerCancellationChild {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.terminate_and_reap();
        }
    }
}

fn verify_owner_cancellation_private_group(identity: process::ProcessIdentity) -> Result<()> {
    anyhow::ensure!(
        identity.observe() == process::ProcessIdentityObservation::Matching,
        OWNER_CANCELLATION_GROUP_DETAIL
    );
    let pid = libc::pid_t::try_from(identity.pid)?;
    let group = unsafe { libc::getpgid(pid) };
    anyhow::ensure!(
        group != OWNER_CANCELLATION_SYSTEM_CALL_FAILURE && group == pid,
        OWNER_CANCELLATION_GROUP_DETAIL
    );
    anyhow::ensure!(
        identity.observe() == process::ProcessIdentityObservation::Matching,
        OWNER_CANCELLATION_GROUP_DETAIL
    );
    Ok(())
}
