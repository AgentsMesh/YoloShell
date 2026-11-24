use crate::{TEST_HARNESS_EXACT_ARGUMENT, TEST_HARNESS_NOCAPTURE_ARGUMENT};
use anyhow::Result;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
use yolo_runtime::{
    paths::{constants::SOCKET_ROOT_ENVIRONMENT, directory::PrivateDirectory},
    process::ScopedWaitableChild,
};

const CHILD_MODE_ENV: &str = "YOLO_ORPHAN_NAMESPACE_LOSS_CHILD";
const CHILD_MODE_VALUE: &str = "1";
const SHORT_ROOT_PREFIX: &str = "yo-";
const SOCKET_ROOT_PARENT: &str = "/tmp";
const CHILD_TIMEOUT: Duration = Duration::from_secs(300);
const TEST_FILTER: &str = concat!(
    "orphan_guard::",
    stringify!(worker_and_child_exit_when_the_session_persistence_root_is_removed)
);

pub(super) fn is_child() -> bool {
    std::env::var_os(CHILD_MODE_ENV).is_some_and(|value| value == CHILD_MODE_VALUE)
}

pub(super) async fn run() -> Result<()> {
    let mut temporary = tempfile::Builder::new()
        .prefix(SHORT_ROOT_PREFIX)
        .tempdir_in(SOCKET_ROOT_PARENT)?;
    // A failed or interrupted child may still own artifacts. Preserve this exact
    // private root until success confirms the original scenario's process teardown.
    temporary.disable_cleanup(true);
    let root = PrivateDirectory::open_or_create_strict(&temporary.path().canonicalize()?)?;
    let deadline = Instant::now() + CHILD_TIMEOUT;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            TEST_HARNESS_EXACT_ARGUMENT,
            TEST_FILTER,
            TEST_HARNESS_NOCAPTURE_ARGUMENT,
        ])
        .env(CHILD_MODE_ENV, CHILD_MODE_VALUE)
        .env(SOCKET_ROOT_ENVIRONMENT, root.path())
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let child = ScopedWaitableChild::spawn_before_deadline(&mut command, deadline)?;
    child.wait_for_clean_exit(deadline).await?;
    root.verify_path()?;
    temporary.close()?;
    Ok(())
}
