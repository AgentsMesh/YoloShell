impl ConcurrentRuntimeErrorProcess {
    fn terminate_and_reap(&mut self) -> Result<()> {
        let mut last_error = self.terminate_private_group().err();
        if let Some(containment) = self.containment.as_mut()
            && let Err(error) = containment.terminate(libc::SIGKILL)
        {
            last_error = Some(error);
        }
        if let Some(child) = self.child.as_mut()
            && !matches!(child.request_force_termination(), Ok(true))
            && let Err(error) = child.request_direct_termination()
        {
            last_error = Some(error.into());
        }
        let deadline = Instant::now() + RUNTIME_ERROR_PROCESS_REAP_TIMEOUT;
        loop {
            let descendants_settled = self.refresh_containment(&mut last_error);
            let child_settled = self.reap_child(&mut last_error);
            if child_settled && descendants_settled {
                self.settled = true;
                return Ok(());
            }
            if Instant::now() >= deadline {
                if let Some(error) = last_error {
                    return Err(error).context(RUNTIME_ERROR_PROCESS_REAP_DETAIL);
                }
                anyhow::bail!(RUNTIME_ERROR_PROCESS_REAP_DETAIL);
            }
            std::thread::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL);
        }
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
                anyhow::bail!(RUNTIME_ERROR_PROCESS_GROUP_UNKNOWN_DETAIL)
            }
        }
    }

    fn poll_until(
        &mut self,
        deadline: Instant,
        detail: &'static str,
    ) -> Result<std::process::ExitStatus> {
        let mut last_error = None;
        loop {
            if let Some(containment) = self.containment.as_mut() {
                let _ = containment.refresh();
            }
            let child = self
                .child
                .as_mut()
                .context(RUNTIME_ERROR_PROCESS_CHILD_DETAIL)?;
            match child.try_wait() {
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
            std::thread::sleep(RUNTIME_ERROR_CUSTODY_POLL_INTERVAL);
        }
    }

    fn refresh_containment(&mut self, last_error: &mut Option<anyhow::Error>) -> bool {
        let Some(containment) = self.containment.as_mut() else {
            return true;
        };
        if let Err(error) = containment.refresh() {
            *last_error = Some(error);
            return false;
        }
        match containment.live_identities() {
            Ok(identities) => identities.is_empty(),
            Err(error) => {
                *last_error = Some(error);
                false
            }
        }
    }

    fn reap_child(&mut self, last_error: &mut Option<anyhow::Error>) -> bool {
        let Some(child) = self.child.as_mut() else {
            return true;
        };
        match child.try_wait() {
            Ok(Some(_)) => {
                self.child.take();
                true
            }
            Ok(None) => false,
            Err(error) => {
                *last_error = Some(error.into());
                false
            }
        }
    }
}

impl Drop for ConcurrentRuntimeErrorProcess {
    fn drop(&mut self) {
        if !self.settled {
            let _ = self.terminate_and_reap();
        }
    }
}

fn verify_runtime_error_private_group(identity: process::ProcessIdentity) -> Result<()> {
    anyhow::ensure!(
        identity.observe() == process::ProcessIdentityObservation::Matching,
        RUNTIME_ERROR_PROCESS_GROUP_DETAIL
    );
    let pid = libc::pid_t::try_from(identity.pid)?;
    let group = unsafe { libc::getpgid(pid) };
    anyhow::ensure!(
        group != RUNTIME_ERROR_SYSTEM_CALL_FAILURE && group == pid,
        RUNTIME_ERROR_PROCESS_GROUP_DETAIL
    );
    anyhow::ensure!(
        identity.observe() == process::ProcessIdentityObservation::Matching,
        RUNTIME_ERROR_PROCESS_GROUP_DETAIL
    );
    Ok(())
}
