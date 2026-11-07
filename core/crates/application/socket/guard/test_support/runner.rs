use super::constants::*;
use super::custody::{ChildCustody, RootCustody, TreeCustody};
use super::environment::{canonical_executable, resolve_executable};
use super::launcher::LauncherClient;
use super::watchdog_client::WatchdogClient;
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;
use std::process::{ExitCode, ExitStatus};
use yolo_application_support::isolated_test_child;
use yolo_process::ProcessIdentity;

pub(super) fn run_guard() -> Result<ExitCode> {
    let executable = canonical_executable(&resolve_executable()?)?;
    let mut watchdog = WatchdogClient::spawn()?;
    let mut socket_root = RootCustody::adopt(watchdog.socket_root())?;
    let mut launcher = LauncherClient::spawn(
        &executable,
        watchdog.socket_root(),
        watchdog.process_discovery(),
        harness_arguments(),
    )?;
    let process_root = launcher.identity();
    watchdog.track(process_root)?;
    #[cfg(target_os = "macos")]
    launcher.commit_discovery(watchdog.process_discovery())?;
    let mut tree = TreeCustody::capture(
        process_root,
        watchdog.process_discovery().clone(),
        ProcessIdentity::capture(std::process::id())?,
    )?;
    let child = launcher.start(watchdog.capability())?;
    let mut child = ChildCustody::new(child, process_root);
    let status = monitor(&mut child, &mut tree, &mut watchdog);
    let status = settle(
        status,
        &mut child,
        &mut tree,
        &mut watchdog,
        &mut socket_root,
    )?;
    Ok(isolated_test_child::exit_code(status))
}

fn harness_arguments() -> Vec<OsString> {
    let mut arguments = std::env::args_os()
        .skip(PROGRAM_NAME_ARGUMENTS)
        .collect::<Vec<_>>();
    if let Some(filter) =
        std::env::var_os(TEST_FILTER_ENVIRONMENT).filter(|value| !value.is_empty())
    {
        arguments.push(filter);
    }
    arguments
}

fn monitor(
    child: &mut ChildCustody,
    tree: &mut TreeCustody,
    watchdog: &mut WatchdogClient,
) -> Result<ExitStatus> {
    loop {
        #[cfg(feature = "adversarial-test-hooks")]
        super::fault::check();
        if let Some(status) = watchdog.exited_early()? {
            anyhow::bail!(
                "{WATCHDOG_EARLY_EXIT_DETAIL}: {}",
                WatchdogClient::describe_exit_status(status)
            );
        }
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        tree.refresh().context(WATCHDOG_REFRESH_DETAIL)?;
        std::thread::sleep(PROCESS_POLL_INTERVAL);
    }
}

fn settle(
    status: Result<ExitStatus>,
    child: &mut ChildCustody,
    tree: &mut TreeCustody,
    watchdog: &mut WatchdogClient,
    root: &mut RootCustody,
) -> Result<ExitStatus> {
    let mut failures = Vec::new();
    let status = match status {
        Ok(status) => Some(status),
        Err(error) => {
            failures.push(format!("{error:#}"));
            None
        }
    };
    let watchdog_finished = match watchdog.finish() {
        Ok(()) => {
            root.disarm();
            true
        }
        Err(error) => {
            failures.push(format!("{error:#}"));
            false
        }
    };
    record(&mut failures, tree.settle());
    record(&mut failures, child.settle());
    if !watchdog_finished {
        record(&mut failures, root.finalize());
    }
    ensure!(
        failures.is_empty(),
        "{RUNNER_SETTLEMENT_DETAIL}: {}",
        failures.join(FAILURE_SEPARATOR)
    );
    status.context(EXECUTION_CONTEXT)
}

fn record(failures: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        failures.push(format!("{error:#}"));
    }
}
