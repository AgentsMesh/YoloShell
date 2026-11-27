use super::{super::super::local_startup_support::*, support::*};
use std::process::{Command, Stdio};
use yolo_runtime::process::ScopedWaitableChild;

#[tokio::test(flavor = "multi_thread")]
async fn bootstrap_termination_signals_retire_the_unready_detached_child() {
    for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP, libc::SIGQUIT] {
        let directory = tempfile::tempdir_in(TEMP_ROOT).unwrap();
        let root = directory.path().canonicalize().unwrap();
        let home = root.join(HOST_DIRECTORY);
        let state = root.join(STATE_DIRECTORY);
        let blocked_host = wrapper(&root, BLOCKED_SCRIPT);
        let mut command = Command::new(hub_binary());
        command
            .args(yolo_launch_contract::ProcessRole::Hub.arguments())
            .arg(BOOTSTRAP)
            .arg(STATE)
            .arg(&state)
            .arg(HOST_HOME)
            .arg(&home)
            .arg(HOST_EXECUTABLE)
            .arg(blocked_host)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = ScopedWaitableChild::spawn_detached_before_deadline(
            &mut command,
            deadline().into_std(),
        )
        .unwrap();
        let bootstrap = crate::ProcessIdentity::capture(child.process_id()).unwrap();
        let pid_file = root.join(BLOCKED_PID);
        wait_file(&pid_file).await;
        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .parse()
            .expect(PID_PARSE);
        let blocked = ExactCleanup(crate::ProcessIdentity::capture(pid).unwrap());
        assert!(bootstrap.signal(signal).unwrap());
        let status = tokio::time::timeout(SETTLE_TIMEOUT, async {
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    break status;
                }
                tokio::time::sleep(POLL).await;
            }
        })
        .await
        .unwrap();
        assert!(!status.success());
        child.retire_reaped().unwrap();
        tokio::time::timeout(SETTLE_TIMEOUT, crate::wait_for_identity_gone(blocked.0))
            .await
            .unwrap();
    }
}
