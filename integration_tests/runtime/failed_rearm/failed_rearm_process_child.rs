use super::constants::*;
use anyhow::{Context, Result, ensure};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use yolo_host::failed_rearm_process_fixture::{Boundary, CRASH_BOUNDARY_ENVIRONMENT};

#[path = "failed_rearm_process_child_custody.rs"]
mod custody;
#[cfg(test)]
#[path = "failed_rearm_process_child_tests.rs"]
mod tests;

pub(super) fn prepare(home: &Path) -> Result<String> {
    let completed = run(PREPARE_MODE, home, None, None)?;
    completed.require_success()?;
    ensure!(completed.stderr.is_empty(), FIXTURE_OUTPUT_RESIDUE);
    let evidence = String::from_utf8(completed.stdout).context(FIXTURE_OUTPUT_ENCODING)?;
    ensure!(!evidence.is_empty(), FIXTURE_OUTPUT_EMPTY);
    Ok(evidence)
}

pub(super) fn crash(home: &Path, boundary: Boundary, evidence: &str) -> Result<()> {
    let completed = run(CRASH_MODE, home, Some(boundary), Some(evidence))?;
    completed.require_empty_output()?;
    ensure!(
        completed.status.code().is_none() && completed.status.signal() == Some(libc::SIGKILL),
        "{FIXTURE_CHILD_STATUS}: {}",
        completed.status
    );
    Ok(())
}

pub(super) fn recover(home: &Path, boundary: Boundary, evidence: &str) -> Result<()> {
    let completed = run(RECOVER_MODE, home, Some(boundary), Some(evidence))?;
    completed.require_empty_output()?;
    completed.require_success()
}

pub(super) fn assert_no_process_residue(home: &Path) -> Result<()> {
    let mut command = fixture_command(PROCESS_LIST_MODE)?;
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let completed = custody::spawn(&mut command)?.wait()?;
    completed.require_success()?;
    ensure!(completed.stderr.is_empty(), PROCESS_LIST_FAILURE);
    let listing = String::from_utf8(completed.stdout).context(PROCESS_LIST_FAILURE)?;
    let home = home.to_string_lossy();
    ensure!(
        listing.lines().all(|line| !line.contains(home.as_ref())),
        PROCESS_RESIDUE
    );
    Ok(())
}

fn run(
    mode: &str,
    home: &Path,
    boundary: Option<Boundary>,
    evidence: Option<&str>,
) -> Result<custody::Completed> {
    let mut command = fixture_command(mode)?;
    command
        .arg(home)
        .env_remove(CRASH_BOUNDARY_ENVIRONMENT)
        .stdin(if evidence.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(boundary) = boundary {
        command.arg(boundary.as_str());
        if mode == CRASH_MODE {
            command.env(CRASH_BOUNDARY_ENVIRONMENT, boundary.as_str());
        }
    }
    let mut child = custody::spawn(&mut command)?;
    if let Some(input) = evidence {
        child.write_input(input.as_bytes())?;
    }
    child.wait()
}

fn fixture_command(mode: &str) -> Result<Command> {
    let executable = std::env::var_os(FIXTURE_BINARY_ENVIRONMENT)
        .map(PathBuf::from)
        .context(FIXTURE_BINARY_CONTEXT)?;
    let mut command = Command::new(executable);
    command.arg(mode);
    Ok(command)
}

impl custody::Completed {
    fn require_success(&self) -> Result<()> {
        ensure!(
            self.status.success()
                && self.status.code() == Some(SUCCESS_EXIT_CODE)
                && self.status.signal().is_none(),
            "{FIXTURE_CHILD_STATUS}: {}",
            self.status
        );
        Ok(())
    }

    fn require_empty_output(&self) -> Result<()> {
        ensure!(
            self.stdout.is_empty() && self.stderr.is_empty(),
            FIXTURE_OUTPUT_RESIDUE
        );
        Ok(())
    }
}
