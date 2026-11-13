use super::process_assertions::{
    assert_process_exits, assert_process_tree_exits, wait_for_root_exit,
};
use super::tests::{
    TEST_MARKER_FILE, production_fixture, spawn_test_guard, wait_for_descendant,
    wait_for_tracked_identity,
};
use super::*;
use std::process::Command;
use std::sync::Arc;
const CLEANUP_TIMEOUT: std::time::Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;
const TEST_ROOT_EXIT_EXTENSION: &str = "exit";
const CLEANUP_COMPLETION_DETAIL: &str = "startup fault cleanup did not publish completion";
const CLEANUP_STATE_DETAIL: &str = "latest fault-reaper state";
const EARLY_IDENTITY_EXECUTABLE: &str = "/bin/sleep";
const EARLY_IDENTITY_DURATION: &str = "30";

struct DescendantCleanup(ProcessIdentity);

impl Drop for DescendantCleanup {
    fn drop(&mut self) {
        let _ = self.0.enforce_exact_test_cleanup();
    }
}

#[tokio::test]
async fn failed_startup_kills_setsid_hup_resistant_descendant() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let production_fixture::HeldStartup {
        mut resources,
        root,
        release,
        cleanup_scope,
    } = production_fixture::spawn(&marker);
    resources.child_mut().stop_monitor().unwrap();
    release.release();
    let descendant = wait_for_descendant(&marker);
    let _descendant_cleanup = DescendantCleanup(descendant);
    assert_is_independent_session(descendant);
    let (completion, state) = {
        let reaper = resources.child_mut().fault_reaper.as_mut().unwrap();
        (
            reaper.observe_startup_completion(),
            reaper.observe_state_for_test(),
        )
    };
    drop(resources);
    let outcome = completion
        .receive_timeout(CLEANUP_TIMEOUT)
        .unwrap_or_else(|error| {
            panic!(
                "{CLEANUP_COMPLETION_DETAIL}: {error:?}; {CLEANUP_STATE_DETAIL}: {:#?}",
                state.latest()
            )
        });
    assert!(outcome.fault.is_none(), "{:?}", outcome.fault);
    cleanup_scope.settle().await.unwrap();
    assert_process_tree_exits(root, descendant);
}

#[tokio::test]
async fn scope_refresh_uses_retained_anchor_after_root_exit() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let production_fixture::HeldStartup {
        mut resources,
        root,
        release,
        cleanup_scope,
    } = production_fixture::spawn(&marker);
    release.release();
    let descendant = wait_for_descendant(&marker);
    let _descendant_cleanup = DescendantCleanup(descendant);
    let tree = Arc::clone(resources.child().tree.as_ref().unwrap());
    wait_for_tracked_identity(resources.child(), descendant);
    std::fs::write(marker.with_extension(TEST_ROOT_EXIT_EXTENSION), []).unwrap();
    wait_for_root_exit(root);
    let refreshed = refresh(&tree).unwrap();
    assert!(refreshed.contains(&descendant));
    #[cfg(not(target_os = "linux"))]
    {
        descendant.enforce_exact_test_cleanup().unwrap();
        assert_process_exits(descendant);
    }
    let outcome = crate::worker::pty_child_reaper::wait_direct_child(root.pid);
    assert!(outcome.status.is_some());
    let (completion, state) = {
        let reaper = resources.child_mut().fault_reaper.as_mut().unwrap();
        (
            reaper.observe_startup_completion(),
            reaper.observe_state_for_test(),
        )
    };
    drop(tree);
    drop(resources);
    let outcome = completion
        .receive_timeout(CLEANUP_TIMEOUT)
        .unwrap_or_else(|error| {
            panic!(
                "{CLEANUP_COMPLETION_DETAIL}: {error:?}; {CLEANUP_STATE_DETAIL}: {:#?}",
                state.latest()
            )
        });
    assert!(outcome.status.is_none());
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::LostUnknown
    );
    let fault = outcome.fault.unwrap();
    assert!(crate::process::is_wait_authority_loss(&fault), "{fault:#}");
    cleanup_scope.settle().await.unwrap();
    assert_process_exits(descendant);
}

#[test]
fn early_live_identity_settles_cleanup_after_external_wait_authority_loss() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(EARLY_IDENTITY_EXECUTABLE).arg(EARLY_IDENTITY_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let output_calls = Arc::new(std::sync::atomic::AtomicUsize::default());
    let mut guard = StartupChildGuard::with_output(
        Box::new(child),
        ExclusivePtyOutput::observed(Arc::clone(&output_calls)),
    );
    assert_eq!(guard.identity, Some(root));
    assert!(guard.live_identity.as_ref().unwrap().try_matches().unwrap());
    assert!(root.signal(libc::SIGKILL).unwrap());
    assert!(
        crate::worker::pty_child_reaper::wait_direct_child(root.pid)
            .status
            .is_some()
    );

    assert!(guard.terminate_untracked_at_deadline_for_test().is_err());

    assert!(!guard.requires_cleanup());
    assert!(output_calls.load(std::sync::atomic::Ordering::Acquire) > usize::default());
}

fn assert_is_independent_session(identity: ProcessIdentity) {
    let pid = i32::try_from(identity.pid).unwrap();
    assert_eq!(unsafe { libc::getsid(pid) }, pid);
    assert_eq!(unsafe { libc::getpgid(pid) }, pid);
}
