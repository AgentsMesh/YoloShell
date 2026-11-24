use crate::process::captured_tree::gone;
use crate::process::{CapturedProcessTree, ProcessIdentity};
use std::process::{Child, Command};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const REPORT_FAILURE_DETAIL: &str = "injected tracked-producers report failure";
const REPORT_UNEXPECTEDLY_SUCCEEDED_DETAIL: &str = "failed report consumed gone authority";
const REPORT_OUTPUT: &str = "reported";

struct ChildGuard {
    child: Child,
    identity: ProcessIdentity,
}

impl ChildGuard {
    fn spawn() -> (Self, CapturedProcessTree) {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(SLEEP_EXECUTABLE).arg(SLEEP_DURATION),
        );
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(identity).unwrap();
        (Self { child, identity }, tree)
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.identity.signal(libc::SIGKILL);
        let _ = self.child.wait();
    }
}

#[test]
fn opaque_report_never_detaches_authority_from_its_tree() {
    let (_owner, owner_tree) = ChildGuard::spawn();
    let (_foreign, foreign_tree) = ChildGuard::spawn();
    let owner_root = owner_tree.root();
    let foreign_root = foreign_tree.root();
    let completion = gone::complete(owner_tree);

    let error = match completion.report::<()>(|_authority| anyhow::bail!(REPORT_FAILURE_DETAIL)) {
        Err(error) => error,
        Ok(_) => panic!("{REPORT_UNEXPECTEDLY_SUCCEEDED_DETAIL}"),
    };
    assert!(error.error().to_string().contains(REPORT_FAILURE_DETAIL));
    let completion = error.into_state();
    assert_eq!(completion.root(), owner_root);
    assert_ne!(completion.root(), foreign_root);

    let (reported, output) = completion
        .report(|authority| {
            assert_eq!(authority.root(), owner_root);
            Ok(REPORT_OUTPUT)
        })
        .unwrap();
    assert_eq!(output, REPORT_OUTPUT);
    assert_eq!(reported.root(), owner_root);
    assert!(foreign_root.matches());
    let reported = reported.retire_direct_for_test();
    assert!(reported.session_is_retired());
}
