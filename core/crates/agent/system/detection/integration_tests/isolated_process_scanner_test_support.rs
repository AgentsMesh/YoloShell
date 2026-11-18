use super::IsolatedProcessScanner;
use crate::process::ProcessIdentity;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::time::Duration;

const TEST_RUNTIME_ENVIRONMENT: &str = "YOLO_TEST_RUNTIME";
const TEST_RUNTIME_MISSING: &str = "YOLO_TEST_RUNTIME must name the adversarial yolo runtime";
const ROOT_EXECUTABLE: &str = "/bin/sleep";
const ROOT_DURATION_ARGUMENT: &str = "30";
const CHILD_EXIT_BUDGET: Duration = Duration::from_secs(2);
const CHILD_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
const CHILD_NOT_REAPED: &str = "cancelled isolated scanner helper was not reaped";
const WAITID_FAILURE: libc::c_int = -1;
const WAITID_SUCCESS: libc::c_int = 0;
const WAITID_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;

pub(super) fn runtime_executable() -> PathBuf {
    std::env::var_os(TEST_RUNTIME_ENVIRONMENT)
        .expect(TEST_RUNTIME_MISSING)
        .into()
}

pub(super) fn scanner() -> IsolatedProcessScanner {
    IsolatedProcessScanner::new(runtime_executable(), Vec::new()).unwrap()
}

pub(super) async fn await_child_reaped(process_id: u32) {
    tokio::time::timeout(CHILD_EXIT_BUDGET, async {
        while !wait_authority_is_gone(process_id) {
            tokio::time::sleep(CHILD_EXIT_POLL_INTERVAL).await;
        }
    })
    .await
    .expect(CHILD_NOT_REAPED);
}

fn wait_authority_is_gone(process_id: u32) -> bool {
    let mut information = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            information.as_mut_ptr(),
            WAITID_OPTIONS,
        )
    };
    if result == WAITID_SUCCESS {
        return false;
    }
    assert_eq!(result, WAITID_FAILURE);
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
    true
}

pub(super) struct TestRoot {
    child: Option<Child>,
    pub(super) identity: ProcessIdentity,
}

impl TestRoot {
    pub(super) fn spawn() -> Self {
        let child = crate::process::spawn_detached(
            Path::new(ROOT_EXECUTABLE),
            &[OsString::from(ROOT_DURATION_ARGUMENT)],
            crate::process::DetachedStandardError::Null,
        )
        .unwrap();
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        Self {
            child: Some(child),
            identity,
        }
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        if let Some(child) = self.child.take() {
            crate::process::terminate_detached_child(child).unwrap();
        }
    }
}
