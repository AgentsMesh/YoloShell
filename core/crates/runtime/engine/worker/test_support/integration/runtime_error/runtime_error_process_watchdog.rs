use crate::worker::tests::*;
include!("runtime_error_process_output.rs");
const RUNTIME_ERROR_PROCESS_PROTOCOL_TIMEOUT: Duration = RUNTIME_ERROR_CUSTODY_TIMEOUT;
const RUNTIME_ERROR_PROCESS_JOIN_TIMEOUT: Duration = RUNTIME_ERROR_CUSTODY_TIMEOUT;
const RUNTIME_ERROR_PROCESS_WATCHDOG_TIMEOUT: Duration = RUNTIME_ERROR_RECORD_TIMEOUT
    .saturating_add(RUNTIME_ERROR_PROCESS_PROTOCOL_TIMEOUT)
    .saturating_add(RUNTIME_ERROR_PROCESS_JOIN_TIMEOUT);
const RUNTIME_ERROR_PROCESS_REAP_TIMEOUT: Duration = RUNTIME_ERROR_CUSTODY_TIMEOUT;
const RUNTIME_ERROR_PRIVATE_PROCESS_GROUP: i32 = 0;
const RUNTIME_ERROR_SYSTEM_CALL_FAILURE: libc::pid_t = -1;
const RUNTIME_ERROR_PROCESS_EXECUTABLE_DETAIL: &str =
    "resolve concurrent runtime-error test executable";
const RUNTIME_ERROR_PROCESS_SPAWN_DETAIL: &str = "spawn concurrent runtime-error process";
pub(super) const RUNTIME_ERROR_PROCESS_IDENTITY_DETAIL: &str =
    "concurrent runtime-error process identity is unavailable";
const RUNTIME_ERROR_PROCESS_ALIAS_DETAIL: &str =
    "concurrent runtime-error process aliases its parent";
const RUNTIME_ERROR_PROCESS_GROUP_DETAIL: &str =
    "concurrent runtime-error process did not enter its private process group";
const RUNTIME_ERROR_PROCESS_GROUP_UNKNOWN_DETAIL: &str =
    "concurrent runtime-error process group identity is unknown";
pub(super) const RUNTIME_ERROR_PROCESS_TIMEOUT_DETAIL: &str =
    "concurrent runtime-error process exceeded its lifecycle watchdog";
const RUNTIME_ERROR_PROCESS_REAP_DETAIL: &str =
    "concurrent runtime-error process tree did not settle after termination";
const RUNTIME_ERROR_PROCESS_CHILD_DETAIL: &str =
    "concurrent runtime-error direct child custody is unavailable";

pub(super) struct ConcurrentRuntimeErrorProcess {
    child: Option<process::DetachedChildCustody>,
    pub(super) identity: Option<process::ProcessIdentity>,
    containment: Option<process::CapturedProcessTree>,
    pub(super) deadline: Instant,
    output: RuntimeErrorProcessOutput,
    settled: bool,
}

impl ConcurrentRuntimeErrorProcess {
    fn spawn() -> Result<Self> {
        let mut command = Self::test_command(RUNTIME_ERROR_PROCESS_HELPER_TEST)?;
        command.env(
            RUNTIME_ERROR_PROCESS_HELPER_ENV,
            RUNTIME_ERROR_PROCESS_HELPER_ROLE,
        );
        Self::spawn_command(command)
    }

    pub(super) fn test_command(test: &'static str) -> Result<std::process::Command> {
        let executable =
            std::env::current_exe().context(RUNTIME_ERROR_PROCESS_EXECUTABLE_DETAIL)?;
        let mut command = std::process::Command::new(executable);
        command.args([
            crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
            test,
            crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
            crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ]);
        Ok(command)
    }

    pub(super) fn spawn_command(mut command: std::process::Command) -> Result<Self> {
        use std::os::unix::process::CommandExt as _;
        command.process_group(RUNTIME_ERROR_PRIVATE_PROCESS_GROUP);
        let output = RuntimeErrorProcessOutput::capture(&mut command)?;
        let deadline = Instant::now() + RUNTIME_ERROR_PROCESS_WATCHDOG_TIMEOUT;
        let spawn_guard = process::waitable_child_spawn_guard()?;
        let child = command
            .spawn()
            .context(RUNTIME_ERROR_PROCESS_SPAWN_DETAIL)?;
        drop(spawn_guard);
        let child = process::DetachedChildCustody::capture(child)
            .unwrap_or_else(process::DetachedChildCustodyError::into_custody);
        let identity = child.identity();
        let mut guarded = Self {
            child: Some(child),
            identity,
            containment: None,
            deadline,
            output,
            settled: false,
        };
        let identity = identity.context(RUNTIME_ERROR_PROCESS_IDENTITY_DETAIL)?;
        anyhow::ensure!(
            identity.pid != std::process::id(),
            RUNTIME_ERROR_PROCESS_ALIAS_DETAIL
        );
        verify_runtime_error_private_group(identity)?;
        guarded.containment = Some(process::CapturedProcessTree::capture(identity)?);
        Ok(guarded)
    }

    pub(super) fn finish(mut self) -> Result<()> {
        match self.poll_until(self.deadline, RUNTIME_ERROR_PROCESS_TIMEOUT_DETAIL) {
            Ok(status) if status.success() => {
                self.settled = true;
                Ok(())
            }
            Ok(status) => self.fail(anyhow::anyhow!(
                "{RUNTIME_ERROR_PROCESS_FAILURE_DETAIL}: {status}"
            )),
            Err(error) => self.fail(error),
        }
    }

    fn fail(&mut self, error: anyhow::Error) -> Result<()> {
        let cleanup = self.terminate_and_reap();
        let diagnostics = self.output.diagnostics();
        match cleanup {
            Ok(()) => anyhow::bail!("{error:#}; {diagnostics}"),
            Err(cleanup) => anyhow::bail!("{error:#}; cleanup: {cleanup:#}; {diagnostics}"),
        }
    }
}

include!("runtime_error_process_cleanup.rs");
