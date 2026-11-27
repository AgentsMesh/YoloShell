use super::{capture, constants::*};
use std::{
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_process::ScopedWaitableChild;

#[tokio::test]
async fn incomplete_conflicting_and_bad_profiles_exit_without_terminal_or_workspace_effects() {
    let root = tempfile::tempdir().unwrap();
    let malformed = root.path().join(MALFORMED_PROFILE_FILE);
    fs::write(&malformed, b"not json").unwrap();
    fs::set_permissions(&malformed, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let cases: Vec<(Vec<OsString>, i32)> = vec![
        (vec![RETIRED_PICKER_COMMAND.into()], USAGE_FAILURE),
        (vec![RETIRED_PANELS_COMMAND.into()], USAGE_FAILURE),
        (vec![HUB_ARGUMENT.into(), HUB.into()], USAGE_FAILURE),
        (vec![SOCKET_ARGUMENT.into(), SOCKET.into()], USAGE_FAILURE),
        (
            vec![ENDPOINT_ARGUMENT.into(), LOCAL_ENDPOINT.into()],
            USAGE_FAILURE,
        ),
        (
            vec![
                HUB_ARGUMENT.into(),
                INVALID_HUB.into(),
                SOCKET_ARGUMENT.into(),
                SOCKET.into(),
            ],
            USAGE_FAILURE,
        ),
        (
            vec![
                LOCAL_PROFILE_ARGUMENT.into(),
                malformed.clone().into_os_string(),
            ],
            APPLICATION_FAILURE,
        ),
        (
            vec![
                HUB_ARGUMENT.into(),
                HUB.into(),
                SOCKET_ARGUMENT.into(),
                SOCKET.into(),
                LOCAL_PROFILE_ARGUMENT.into(),
                malformed.into_os_string(),
            ],
            USAGE_FAILURE,
        ),
    ];
    for (arguments, expected) in cases {
        let workspace = root.path().join(WORKSPACE_NAME);
        let stdout = tempfile::NamedTempFile::new_in(root.path()).unwrap();
        let stderr = tempfile::NamedTempFile::new_in(root.path()).unwrap();
        let mut command = Command::new(std::env::var_os(BINARY_ENV).unwrap());
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(stdout.reopen().unwrap())
            .stderr(stderr.reopen().unwrap());
        let deadline = Instant::now() + PROCESS_DEADLINE;
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
        assert_eq!(status.code(), Some(expected));
        assert!(capture(&stdout).is_empty());
        assert!(!capture(&stderr).is_empty());
        assert!(!workspace.exists());
    }
}
