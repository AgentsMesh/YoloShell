#[cfg(target_os = "macos")]
struct FixturePtyCapabilityHolder {
    _directory: tempfile::TempDir,
    child: Option<std::process::Child>,
    identity: Option<process::ProcessIdentity>,
    ready: PathBuf,
    release: PathBuf,
}

#[cfg(target_os = "macos")]
impl FixturePtyCapabilityHolder {
    fn launch(
        writer: process::MacosProcessCapabilityChildWriter,
        authority: &process::MacosProcessCapabilityManagerReader,
    ) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let ready = directory.path().join(FIXTURE_CAPABILITY_HOLDER_READY_FILE);
        let release = directory
            .path()
            .join(FIXTURE_CAPABILITY_HOLDER_RELEASE_FILE);
        let mut command = std::process::Command::new(std::env::current_exe()?);
        command.args([
            BOOTSTRAP_TEST_EXACT_ARGUMENT,
            FIXTURE_CAPABILITY_HOLDER_TEST,
            BOOTSTRAP_TEST_NOCAPTURE_ARGUMENT,
            crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ]);
        command.env(
            FIXTURE_CAPABILITY_HOLDER_ROLE_ENV,
            FIXTURE_CAPABILITY_HOLDER_ROLE,
        );
        command.env(
            FIXTURE_CAPABILITY_HOLDER_DESCRIPTOR_ENV,
            writer.descriptor().to_string(),
        );
        command.env(FIXTURE_CAPABILITY_HOLDER_READY_ENV, &ready);
        command.env(FIXTURE_CAPABILITY_HOLDER_RELEASE_ENV, &release);
        let parent = process::ProcessIdentity::capture(std::process::id())?;
        command.env(
            FIXTURE_CAPABILITY_HOLDER_PARENT_PID_ENV,
            parent.pid.to_string(),
        );
        command.env(
            FIXTURE_CAPABILITY_HOLDER_PARENT_START_ENV,
            parent.start_time.to_string(),
        );
        command.stdin(std::process::Stdio::null());

        let spawn_guard = process::waitable_child_spawn_guard()?;
        writer.configure_exec_inheritance_for_test(&mut command);
        let child = command.spawn()?;
        let mut holder = Self {
            _directory: directory,
            child: Some(child),
            identity: None,
            ready,
            release,
        };
        let preparation = (|| -> Result<()> {
            let identity = process::ProcessIdentity::capture(holder.child_pid())?;
            holder.identity = Some(identity);
            let metadata = process::kernel_process_metadata_for_test(identity.pid)?;
            anyhow::ensure!(
                metadata.start_time == identity.start_time,
                FIXTURE_CAPABILITY_HOLDER_IDENTITY_DETAIL
            );
            let live = process::LiveProcessIdentity::from_macos_unique_id_for_test(
                identity.pid,
                metadata.unique_id,
            )?;
            holder.wait_until_ready()?;
            authority.verify_writer_holder_for_test(&live)?;
            Ok(())
        })();
        drop(writer);
        drop(spawn_guard);
        if let Err(error) = preparation {
            holder.settle_on_drop();
            return Err(error);
        }
        Ok(holder)
    }

    fn child_pid(&self) -> u32 {
        self.child.as_ref().unwrap().id()
    }

    fn identity(&self) -> process::ProcessIdentity {
        self.identity
            .expect(FIXTURE_CAPABILITY_HOLDER_IDENTITY_DETAIL)
    }

    fn wait_until_ready(&mut self) -> Result<()> {
        let deadline =
            Instant::now() + crate::session_timing::SESSION_DURABLE_STARTUP_SETTLEMENT_MARGIN;
        loop {
            if self.ready.try_exists()? {
                return Ok(());
            }
            if let Some(status) = self.child.as_mut().unwrap().try_wait()? {
                self.child.take();
                anyhow::bail!("{FIXTURE_CAPABILITY_HOLDER_EARLY_EXIT_DETAIL}: {status}");
            }
            anyhow::ensure!(
                Instant::now() < deadline,
                FIXTURE_CAPABILITY_HOLDER_READY_DETAIL
            );
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }

    fn release_and_reap(&mut self) -> Result<()> {
        use std::os::unix::process::ExitStatusExt as _;
        std::fs::write(&self.release, EMPTY_MARKER)?;
        let status = self.wait_for_exit(crate::session_timing::HOST_PROCESS_EXIT_TIMEOUT)?;
        let cleanup_signal = matches!(status.signal(), Some(libc::SIGTERM) | Some(libc::SIGKILL));
        anyhow::ensure!(
            status.success() || cleanup_signal,
            "{FIXTURE_CAPABILITY_HOLDER_EXIT_DETAIL}: {status}"
        );
        Ok(())
    }

    fn wait_for_exit(&mut self, timeout: Duration) -> Result<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.as_mut().unwrap().try_wait()? {
                self.child.take();
                return Ok(status);
            }
            anyhow::ensure!(
                Instant::now() < deadline,
                FIXTURE_CAPABILITY_HOLDER_EXIT_DETAIL
            );
            std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
        }
    }

    fn settle_on_drop(&mut self) {
        if self.child.is_none() {
            return;
        }
        let _ = std::fs::write(&self.release, EMPTY_MARKER);
        if self
            .wait_for_exit(crate::session_timing::HOST_PROCESS_EXIT_TIMEOUT)
            .is_ok()
        {
            return;
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }
        let _ = self.wait_for_exit(crate::session_timing::HOST_PROCESS_EXIT_TIMEOUT);
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for FixturePtyCapabilityHolder {
    fn drop(&mut self) {
        self.settle_on_drop();
    }
}

#[cfg(target_os = "macos")]
#[test]
fn fixture_pty_capability_writer_holder() {
    if std::env::var(FIXTURE_CAPABILITY_HOLDER_ROLE_ENV).as_deref()
        != Ok(FIXTURE_CAPABILITY_HOLDER_ROLE)
    {
        return;
    }
    run_fixture_pty_capability_writer_holder().unwrap();
}

#[cfg(target_os = "macos")]
include!("../../test_support/integration/real_pty/real_pty_capability_holder_child.rs");
