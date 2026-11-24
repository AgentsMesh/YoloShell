use super::constants::*;
use super::direct_child::force_unreaped_direct_child;
use super::environment::required_environment;
use super::mode::remove_internal_environment;
use super::protocol::{ControlCapability, write_capability};
use anyhow::{Context, Result};
use std::ffi::{OsStr, OsString};
use std::io::BufReader;
#[cfg(not(target_os = "macos"))]
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::Instant;
use yolo_paths::SOCKET_ROOT_ENVIRONMENT;
#[cfg(target_os = "macos")]
use yolo_process::{PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV, ProcessDiscoveryWriterInheritance};
use yolo_process::{ProcessDiscoveryCapability, ProcessIdentity};
#[path = "launcher_ready.rs"]
mod readiness;
use readiness::{Input, prepare_launcher};

pub(super) struct LauncherClient {
    child: Option<Child>,
    input: Option<Input>,
    identity: Option<ProcessIdentity>,
    #[cfg(target_os = "macos")]
    inheritance: Option<ProcessDiscoveryWriterInheritance>,
}

impl LauncherClient {
    pub(super) fn spawn(
        executable: &Path,
        socket_root: &Path,
        discovery: &ProcessDiscoveryCapability,
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Result<Self> {
        let mut command = Command::new(std::env::current_exe()?);
        command
            .args(arguments)
            .env(GUARD_MODE_ENVIRONMENT, LAUNCHER_MODE)
            .env(LAUNCH_EXECUTABLE_ENVIRONMENT, executable)
            .env(LAUNCH_SOCKET_ROOT_ENVIRONMENT, socket_root)
            .env_remove(TEST_FILTER_ENVIRONMENT)
            .stdin(Stdio::piped());
        #[cfg(feature = "adversarial-test-hooks")]
        super::fault::remove_environment(&mut command);
        #[cfg(target_os = "macos")]
        let inheritance = {
            let inheritance = discovery.begin_writer_inheritance()?;
            inheritance.configure_child(discovery, &mut command)?;
            inheritance
        };
        #[cfg(not(target_os = "macos"))]
        discovery.configure_child_inheritance(&mut command);
        #[cfg(target_os = "linux")]
        let input = readiness::configure(&mut command)?;
        let mut child = command.spawn().context(LAUNCHER_CONTEXT)?;
        #[cfg(not(target_os = "linux"))]
        let input = child.stdin.take().context(LAUNCHER_PIPE_DETAIL)?;
        let mut launcher = Self {
            child: Some(child),
            input: Some(input),
            identity: None,
            #[cfg(target_os = "macos")]
            inheritance: Some(inheritance),
        };
        let identity = ProcessIdentity::capture(launcher.child.as_ref().unwrap().id())?;
        launcher.identity = Some(identity);
        #[cfg(target_os = "linux")]
        readiness::wait(launcher.input.as_mut().unwrap())?;
        Ok(launcher)
    }

    pub(super) fn identity(&self) -> ProcessIdentity {
        self.identity.unwrap()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn commit_discovery(
        &mut self,
        discovery: &ProcessDiscoveryCapability,
    ) -> Result<()> {
        let inheritance = self.inheritance.take().context(LAUNCHER_CONTEXT)?;
        discovery.commit_writer_inheritance(inheritance, self.identity())
    }

    pub(super) fn start(mut self, capability: &ControlCapability) -> Result<Child> {
        write_capability(
            self.input.as_mut().context(LAUNCHER_PIPE_DETAIL)?,
            FRAME_START,
            capability,
        )?;
        self.input.take();
        Ok(self.child.take().unwrap())
    }

    fn abort(&mut self) -> Result<()> {
        self.input.take();
        let mut failure = None;
        if self.reap_until(Instant::now() + LAUNCHER_ABORT_TIMEOUT, &mut failure) {
            return failure.map_or(Ok(()), Err);
        }
        if let Err(error) = force_unreaped_direct_child(
            self.child.as_mut().unwrap(),
            self.identity,
            LAUNCHER_ABORT_DETAIL,
        ) && failure.is_none()
        {
            failure = Some(error);
        }
        if self.reap_until(Instant::now() + LAUNCHER_FORCE_REAP_TIMEOUT, &mut failure) {
            return failure.map_or(Ok(()), Err);
        }
        Err(failure.unwrap_or_else(|| anyhow::anyhow!(LAUNCHER_ABORT_DETAIL)))
    }

    fn reap_until(&mut self, deadline: Instant, failure: &mut Option<anyhow::Error>) -> bool {
        while Instant::now() < deadline {
            match self.child.as_mut().unwrap().try_wait() {
                Ok(Some(_)) => {
                    self.child.take();
                    return true;
                }
                Ok(None) => {}
                Err(error) if failure.is_none() => *failure = Some(error.into()),
                Err(_) => {}
            }
            std::thread::sleep(PROCESS_POLL_INTERVAL);
        }
        false
    }
}

impl Drop for LauncherClient {
    fn drop(&mut self) {
        if self.child.is_some() {
            let _ = self.abort();
        }
    }
}

pub(super) fn run_launcher() -> Result<ExitCode> {
    #[cfg(target_os = "macos")]
    let mut discovery = ProcessDiscoveryCapability::adopt_inherited_writer()?;
    let executable = PathBuf::from(required_environment(LAUNCH_EXECUTABLE_ENVIRONMENT)?);
    let socket_root = PathBuf::from(required_environment(LAUNCH_SOCKET_ROOT_ENVIRONMENT)?);
    #[cfg(target_os = "linux")]
    readiness::announce()?;
    let mut command = prepare_launcher(
        &mut BufReader::new(std::io::stdin()),
        &executable,
        &socket_root,
        std::env::args_os().skip(PROGRAM_NAME_ARGUMENTS),
    )?;
    #[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
    super::fault::forward_discovery_writer_descriptor(&mut command, discovery.descriptor());
    #[cfg(target_os = "macos")]
    let error = discovery.exec_inheriting(&mut command);
    #[cfg(not(target_os = "macos"))]
    let error = anyhow::Error::new(command.exec());
    Err(error).context(EXECUTION_CONTEXT)
}

pub(super) fn child_command<I>(
    executable: &Path,
    socket_root: &Path,
    arguments: I,
    test_filter: Option<OsString>,
) -> Command
where
    I: IntoIterator<Item = OsString>,
{
    let mut command = Command::new(executable);
    command.args(arguments);
    if let Some(test_filter) = test_filter.filter(|value| !value.is_empty()) {
        command.arg(test_filter);
    }
    command
        .env(OsStr::new(SOCKET_ROOT_ENVIRONMENT), socket_root)
        .env_remove(TEST_FILTER_ENVIRONMENT)
        .stdin(Stdio::null());
    remove_internal_environment(&mut command);
    #[cfg(target_os = "macos")]
    command.env_remove(PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV);
    command
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/launcher_tests.rs"]
mod tests;
