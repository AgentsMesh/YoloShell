use super::signal_hook::install_pre_signal_hook;
use super::{CapturedProcessTree, DestructiveSessionUnavailable, ProcessIdentity};
use std::path::Path;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_SECONDS: &str = "30";
const UNEXPECTED_GRACEFUL_SUCCESS_DETAIL: &str =
    "Linux process-tree graceful termination unexpectedly succeeded";
const UNEXPECTED_FORCE_SUCCESS_DETAIL: &str =
    "Linux process-tree force transition unexpectedly succeeded";

struct ChildGuard(Option<Child>);

impl ChildGuard {
    fn spawn() -> (Self, CapturedProcessTree, ProcessIdentity) {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION_SECONDS),
        );
        let identity = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(identity).unwrap();
        (Self(Some(child)), tree, identity)
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
fn graceful_termination_without_broker_authority_delivers_no_signal() {
    let (_child, mut tree, root) = ChildGuard::spawn();
    let signals = signal_counter();
    let observed = Arc::clone(&signals);
    let _hook = install_pre_signal_hook(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });

    let error = tree
        .request_graceful_termination()
        .expect_err(UNEXPECTED_GRACEFUL_SUCCESS_DETAIL);

    assert_unavailable(&error);
    assert_eq!(signals.load(Ordering::SeqCst), usize::default());
    assert!(root.matches());
}

#[test]
fn force_without_broker_authority_fails_before_stop_or_kill() {
    let (_child, tree, root) = ChildGuard::spawn();
    let signals = signal_counter();
    let observed = Arc::clone(&signals);
    let _hook = install_pre_signal_hook(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });

    let error = match tree.begin_force().poll() {
        Err(error) => error,
        Ok(_) => panic!("{UNEXPECTED_FORCE_SUCCESS_DETAIL}"),
    };

    assert_unavailable(error.error());
    assert_eq!(signals.load(Ordering::SeqCst), usize::default());
    assert!(root.matches());
}

fn signal_counter() -> Arc<AtomicUsize> {
    Arc::new(AtomicUsize::new(usize::default()))
}

fn assert_unavailable(error: &anyhow::Error) {
    assert!(
        error
            .downcast_ref::<DestructiveSessionUnavailable>()
            .is_some()
    );
}
