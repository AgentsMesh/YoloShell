use super::super::{constants::*, fixture::Installation};
use crate::hub::federation_support::{POLL, deadline};
use std::{
    fs,
    io::Read,
    process::{Command, Stdio},
};
use tokio::time::{Instant, timeout_at};
use yolo_agent_core::{argv::process_argv_bounded, constants::MAX_SINGLE_PROCESS_ARGV_BYTES};
use yolo_runtime::process::{ScopedWaitableChild, macos_native};

pub(super) async fn cancel(installation: &Installation, signal: i32) {
    let root = installation.profile_path.parent().unwrap();
    let stdout = tempfile::NamedTempFile::new_in(root).unwrap();
    let stderr = tempfile::NamedTempFile::new_in(root).unwrap();
    let mut command = Command::new(&installation.tui);
    command
        .arg(LOCAL_PROFILE_ARGUMENT)
        .arg(&installation.profile_path)
        .env(CLIENT_STATE_ENV, &installation.workspace)
        .stdin(Stdio::null())
        .stdout(stdout.reopen().unwrap())
        .stderr(stderr.reopen().unwrap());
    let end = deadline();
    let mut child =
        ScopedWaitableChild::spawn_detached_before_deadline(&mut command, end.into_std()).unwrap();
    let tui = child
        .captured_identity()
        .expect("owned TUI child requires captured identity");
    let helper = helper_ready(installation, tui, end).await;
    assert!(tui.matches() && helper.matches());
    assert!(tui.signal(signal).unwrap());
    let status = timeout_at(end, async {
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    child.retire_reaped().unwrap();
    assert_eq!(status.code(), Some(CANCELLATION_EXIT_CODE));
    assert!(
        capture(&stdout).is_empty(),
        "bootstrap cancellation must not enter raw terminal output"
    );
    assert_eq!(
        String::from_utf8(capture(&stderr)).unwrap().trim(),
        CANCELLATION_MESSAGE
    );
    timeout_at(end, async {
        while helper.matches() {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    assert!(!tui.matches());
    assert!(!installation.profile.host_home.exists());
    assert!(!installation.workspace.exists());
    let persisted = yolo_local_profile::LocalProfile::load(&installation.profile_path).unwrap();
    assert_eq!(persisted, installation.profile);
}

async fn helper_ready(
    installation: &Installation,
    tui: yolo_runtime::process::ProcessIdentity,
    end: Instant,
) -> crate::ProcessIdentity {
    // This helper has no controlling terminal; foreground-agent scans exclude it.
    assert!(
        macos_native::process_metadata(tui.pid)
            .unwrap()
            .foreground_group
            <= 0
    );
    timeout_at(end, async {
        loop {
            assert!(tui.matches());
            if let Ok(text) = fs::read_to_string(installation.profile.state.join(HELPER_READY_FILE))
            {
                if let Ok(pid) = text.parse::<u32>() {
                    let metadata = macos_native::process_metadata(pid).unwrap();
                    assert_eq!(metadata.parent_pid, tui.pid);
                    assert_eq!(
                        macos_native::executable_path(pid).unwrap(),
                        installation.tui
                    );
                    let helper = crate::ProcessIdentity {
                        pid: metadata.pid,
                        start_time: metadata.start_time,
                    };
                    let observed =
                        process_argv_bounded(helper.pid, MAX_SINGLE_PROCESS_ARGV_BYTES, &[])
                            .unwrap();
                    assert_eq!(macos_native::process_metadata(pid).unwrap(), metadata);
                    assert_eq!(
                        macos_native::executable_path(pid).unwrap(),
                        installation.tui
                    );
                    assert_eq!(
                        observed.values,
                        helper_arguments(installation),
                        "bootstrap helper arguments differ for its captured identity"
                    );
                    assert!(tui.matches() && helper.matches());
                    return helper;
                }
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap()
}

fn helper_arguments(installation: &Installation) -> Vec<String> {
    use crate::hub::local_startup_support::{BOOTSTRAP, HOST_EXECUTABLE, HOST_HOME, STATE};
    let executable = installation.tui.to_str().unwrap();
    std::iter::once(executable)
        .chain(yolo_launch_contract::ProcessRole::Hub.arguments())
        .chain([
            BOOTSTRAP,
            STATE,
            installation.profile.state.to_str().unwrap(),
            HOST_HOME,
            installation.profile.host_home.to_str().unwrap(),
            HOST_EXECUTABLE,
            executable,
        ])
        .map(str::to_owned)
        .collect()
}

fn capture(file: &tempfile::NamedTempFile) -> Vec<u8> {
    let mut bytes = Vec::new();
    file.reopen()
        .unwrap()
        .take(MAX_EVIDENCE_BYTES + OVERFLOW_BYTE)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() as u64 <= MAX_EVIDENCE_BYTES);
    bytes
}
