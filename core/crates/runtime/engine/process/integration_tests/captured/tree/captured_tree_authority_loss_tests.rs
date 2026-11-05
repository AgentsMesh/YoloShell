use super::{CapturedProcessTree, ProcessIdentity};
use std::path::Path;
use std::process::{Child, Command};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const FORBIDDEN_REFRESH_DETAIL: &str = "authority-loss cleanup refreshed process inventory";

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn() -> Self {
        Self(Some(crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
        )))
    }

    fn child(&mut self) -> &mut Child {
        self.0.as_mut().unwrap()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn captured_only_termination_never_refreshes_membership() {
    let mut child = ChildGuard::spawn();
    let identity = ProcessIdentity::capture(child.child().id()).unwrap();
    let mut tree = CapturedProcessTree::capture(identity).unwrap();
    tree.fail_next_refresh(FORBIDDEN_REFRESH_DETAIL);

    assert_eq!(tree.terminate_captured(libc::SIGKILL).unwrap(), [identity]);
    child.child().wait().unwrap();
    child.0 = None;

    let error = tree.refresh().unwrap_err();
    assert!(error.to_string().contains(FORBIDDEN_REFRESH_DETAIL));
}
