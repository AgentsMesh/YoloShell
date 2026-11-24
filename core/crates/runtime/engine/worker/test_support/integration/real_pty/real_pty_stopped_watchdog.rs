struct StoppedOutputChild {
    child: Option<std::process::Child>,
    context: StoppedOutputContext,
    durable: process::ProcessIdentity,
    live: process::LiveProcessIdentity,
}

struct PendingStoppedOutputChild {
    child: Option<std::process::Child>,
    context: StoppedOutputContext,
}

impl StoppedOutputChild {
    fn spawn(mut command: std::process::Command, context: StoppedOutputContext) -> Result<Self> {
        let spawn_guard = process::waitable_child_spawn_guard()?;
        process::configure_detached_command(&mut command);
        let child = command.spawn()?;
        drop(spawn_guard);
        let mut pending = PendingStoppedOutputChild {
            child: Some(child),
            context: context.clone(),
        };
        let durable = process::ProcessIdentity::capture(pending.child_id())?;
        let pid = libc::pid_t::try_from(durable.pid)?;
        anyhow::ensure!(
            unsafe { libc::getsid(pid) } == pid && unsafe { libc::getpgid(pid) } == pid,
            STOPPED_OUTPUT_SESSION_INVALID_DETAIL
        );
        let metadata = process::kernel_process_metadata_for_test(durable.pid)?;
        anyhow::ensure!(
            metadata.start_time == durable.start_time && durable.pid != std::process::id(),
            STOPPED_OUTPUT_CONTEXT_INVALID_DETAIL
        );
        let live = process::LiveProcessIdentity::from_macos_unique_id_for_test(
            durable.pid,
            metadata.unique_id,
        )?;
        Ok(Self {
            child: pending.take(),
            context,
            durable,
            live,
        })
    }

    fn wait_for_exit(&mut self) -> Result<std::process::ExitStatus> {
        self.poll_until(
            STOPPED_OUTPUT_CHILD_TIMEOUT,
            STOPPED_OUTPUT_CHILD_TIMEOUT_DETAIL,
        )
    }

    fn terminate_and_reap(&mut self) -> Result<std::process::ExitStatus> {
        let mut failures = Vec::new();
        record_stopped_output_result(&mut failures, self.context.settle_root());
        record_stopped_output_result(
            &mut failures,
            force_test_process_tree(self.durable, &self.live),
        );
        if self.live.try_matches().unwrap_or(true) {
            record_stopped_output_result(
                &mut failures,
                self.child
                    .as_mut()
                    .unwrap()
                    .kill()
                    .context(STOPPED_OUTPUT_CHILD_KILL_DETAIL),
            );
        }
        let status = self.poll_until(
            STOPPED_OUTPUT_CHILD_REAP_TIMEOUT,
            STOPPED_OUTPUT_CHILD_REAP_DETAIL,
        );
        if failures.is_empty() {
            return status;
        }
        if let Err(error) = status {
            failures.push(error.to_string());
        }
        anyhow::bail!(failures.join(STOPPED_OUTPUT_FAILURE_SEPARATOR))
    }

    fn poll_until(
        &mut self,
        timeout: Duration,
        detail: &'static str,
    ) -> Result<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait()? {
                self.child.take();
                return Ok(status);
            }
            anyhow::ensure!(Instant::now() < deadline, detail);
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }
}

impl PendingStoppedOutputChild {
    fn child_id(&self) -> u32 {
        self.child.as_ref().unwrap().id()
    }

    fn take(&mut self) -> Option<std::process::Child> {
        self.child.take()
    }
}

impl Drop for PendingStoppedOutputChild {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = self.context.settle_root();
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for StoppedOutputChild {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.terminate_and_reap();
        }
    }
}

fn record_stopped_output_result(failures: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        failures.push(error.to_string());
    }
}
