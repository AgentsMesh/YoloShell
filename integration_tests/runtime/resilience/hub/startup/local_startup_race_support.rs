use crate::hub::local_startup_support::*;
use std::{
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::Duration,
};
use yolo_hub_local_startup::Launch;

pub const DELAY_SECONDS: u64 = 7;
pub const SLOW_FIXTURE_SETUP: Duration = Duration::from_secs(DELAY_SECONDS);
pub const POLL: Duration = Duration::from_millis(20);
pub const SETTLE_TIMEOUT: Duration = Duration::from_secs(5);
pub const SCRIPT_MODE: u32 = 0o700;
pub const WRAPPER: &str = "hub-wrapper";
pub const REAL_HUB: &str = "real-hub";
pub const CONTENDER_EXIT: &str = "contender-exited";
pub const BLOCKED_PID: &str = "blocked.pid";
pub const PID_PARSE: &str = "test child identity record";
pub const HUB_READY_FAILURE: &str = "uncommitted Hub did not become ready";
pub const DELAY_PREFIX: &str = "#!/bin/sh\n/bin/sleep ";
pub const DELAY_EXEC: &str = "\nexec \"$(dirname \"$0\")/real-hub\" \"$@\"\n";
pub const CONTENDER_SCRIPT: &str = "#!/bin/sh\n\"$(dirname \"$0\")/real-hub\" \"$@\"\nresult=$?\n: > \"$(dirname \"$0\")/contender-exited\"\nexit \"$result\"\n";
pub const BLOCKED_SCRIPT: &str =
    "#!/bin/sh\nprintf '%s' \"$$\" > \"$(dirname \"$0\")/blocked.pid\"\nexec /bin/sleep 30\n";

pub fn launch(home: &Path, state: &Path, executable: PathBuf) -> Launch {
    Launch {
        state: state.to_path_buf(),
        host_home: home.to_path_buf(),
        host_executable: crate::runtime_binary(),
        hub_executable: executable,
    }
}

pub fn wrapper(root: &Path, script: &str) -> PathBuf {
    wrapper_for(root, script, &hub_binary())
}

pub fn host_wrapper(root: &Path, script: &str) -> PathBuf {
    wrapper_for(root, script, &crate::runtime_binary())
}

fn wrapper_for(root: &Path, script: &str, executable: &Path) -> PathBuf {
    let path = root.join(WRAPPER);
    std::os::unix::fs::symlink(executable, root.join(REAL_HUB)).unwrap();
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(SCRIPT_MODE)).unwrap();
    path
}

pub async fn wait_file(path: &Path) {
    tokio::time::timeout_at(deadline(), async {
        while !path.is_file() {
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
}

pub struct ExactCleanup(pub crate::ProcessIdentity);
impl Drop for ExactCleanup {
    fn drop(&mut self) {
        let _ = self.0.signal(libc::SIGTERM);
    }
}
