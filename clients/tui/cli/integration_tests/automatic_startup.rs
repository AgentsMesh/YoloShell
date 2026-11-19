use super::{capture, constants::*};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_process::ScopedWaitableChild;

const CUSTODY_REMEDY: &str = "delegated cgroup v2 custody";
const START_FAILED: &str = "local Yolo services failed to start";
const FAILURE_DEADLINE: std::time::Duration = std::time::Duration::from_secs(15);

/// A plain shell has no delegated custody. Starting services must fail fast
/// with the remedy instead of waiting for the full startup deadline.
#[tokio::test]
async fn automatic_startup_without_custody_fails_fast_and_actionably() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let profile_root = root.path().join("profile");
    let profile = root.path().join("local-profile.json");
    let state = profile_root.join("hub-local");
    let host_home = profile_root.join("runtime");
    fs::write(
        &profile,
        format!(
            "{{\"schema_version\":1,\"state\":\"{}\",\"host_home\":\"{}\"}}",
            state.display(),
            host_home.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&profile, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();

    let stdout = tempfile::NamedTempFile::new_in(root.path()).unwrap();
    let stderr = tempfile::NamedTempFile::new_in(root.path()).unwrap();
    let mut command = Command::new(std::env::var_os(BINARY_ENV).unwrap());
    command
        .args([LOCAL_PROFILE_ARGUMENT, profile.to_str().unwrap()])
        .env_remove("YOLO_CGROUP_DELEGATED_ROOT")
        .stdin(Stdio::null())
        .stdout(stdout.reopen().unwrap())
        .stderr(stderr.reopen().unwrap());
    let deadline = Instant::now() + FAILURE_DEADLINE;
    let mut child =
        ScopedWaitableChild::spawn_before_deadline(&mut command, deadline.into_std()).unwrap();
    let status = timeout_at(deadline, async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status;
            }
            tokio::time::sleep(PROCESS_POLL).await;
        }
    })
    .await
    .unwrap();
    child.retire_reaped().unwrap();

    assert_eq!(status.code(), Some(APPLICATION_FAILURE));
    assert!(capture(&stdout).is_empty());
    let stderr = String::from_utf8(capture(&stderr)).unwrap();
    assert!(stderr.contains(CUSTODY_REMEDY), "{stderr}");
    assert!(stderr.contains(START_FAILED), "{stderr}");
}
