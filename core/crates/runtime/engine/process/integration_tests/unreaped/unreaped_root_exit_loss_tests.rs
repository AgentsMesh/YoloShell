use super::*;
use crate::process::{CapturedProcessTree, ProcessIdentity, wait_direct_child};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const NON_AUTHORITY_ERROR_DETAIL: &str = "not a wait-authority loss";
const EXIT_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const EXIT_PROOF_POLL_INTERVAL: Duration = Duration::from_millis(10);
const EXIT_PROOF_TIMEOUT_DETAIL: &str = "waitid did not observe the exact root exit";

#[derive(Debug)]
struct ObservedChild {
    child: std::process::Child,
    dropped: Arc<AtomicBool>,
}

impl Drop for ObservedChild {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

impl portable_pty::ChildKiller for ObservedChild {
    fn kill(&mut self) -> std::io::Result<()> {
        portable_pty::ChildKiller::kill(&mut self.child)
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        portable_pty::ChildKiller::clone_killer(&self.child)
    }
}

impl portable_pty::Child for ObservedChild {
    fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        portable_pty::Child::try_wait(&mut self.child)
    }

    fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
        portable_pty::Child::wait(&mut self.child)
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.child.id())
    }
}

struct Fixture {
    root: ProcessIdentity,
    tree: CapturedProcessTree,
    authority: Option<RootWaitAuthority>,
}

impl Fixture {
    fn spawn() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
        );
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(root).unwrap();
        let child = Box::new(child) as crate::process::OwnedPtyChild;
        let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
            .unwrap()
            .bind(child);
        Self {
            root,
            tree,
            authority: Some(authority),
        }
    }

    fn lose(&mut self) -> LostRootWaitAuthority {
        self.authority.take().unwrap().lose(authority_loss())
    }

    fn reap_externally(&self) {
        assert!(self.root.signal(libc::SIGKILL).unwrap());
        wait_direct_child(self.root.pid).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.root.signal(libc::SIGCONT);
        let _ = self.root.signal(libc::SIGKILL);
        let _ = wait_direct_child(self.root.pid);
    }
}

#[test]
fn non_echild_cannot_mint_wait_authority_loss() {
    let error = anyhow!(NON_AUTHORITY_ERROR_DETAIL);

    assert!(WaitAuthorityLoss::from_error(&error).is_none());
}

#[test]
fn exact_root_still_live_cannot_mint_gone_proof() {
    let mut fixture = Fixture::spawn();
    let lost = fixture.lose();

    assert!(lost.observe_gone().unwrap().is_none());
}

#[test]
fn externally_reaped_exact_root_mints_gone_proof() {
    let mut fixture = Fixture::spawn();
    let lost = fixture.lose();
    fixture.reap_externally();

    let proof = lost.observe_gone().unwrap().unwrap();

    assert!(lost.validate(&proof).is_ok());
    fixture.tree.mark_root_gone_without_wait(proof).unwrap();
}

#[test]
fn proof_from_another_exact_root_is_rejected() {
    let mut source = Fixture::spawn();
    let source_lost = source.lose();
    source.reap_externally();
    let proof = source_lost.observe_gone().unwrap().unwrap();
    let mut target = Fixture::spawn();
    let target_lost = target.lose();

    assert!(target_lost.validate(&proof).is_err());
    assert!(target.tree.mark_root_gone_without_wait(proof).is_err());
}

#[test]
fn echild_reap_error_retains_child_custody() {
    let child = crate::process::spawn_waitable_test_child(
        Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
    );
    let root = ProcessIdentity::capture(child.id()).unwrap();
    let tree = CapturedProcessTree::capture(root).unwrap();
    let dropped = Arc::new(AtomicBool::new(false));
    let observed = ObservedChild {
        child,
        dropped: Arc::clone(&dropped),
    };
    let child = Box::new(observed) as crate::process::OwnedPtyChild;
    let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
        .unwrap()
        .bind(child);
    assert!(root.signal(libc::SIGKILL).unwrap());
    let proof = observe_unreaped(&authority);
    wait_direct_child(root.pid).unwrap();

    let error = authority.reap(proof).unwrap_err();

    assert_eq!(
        error
            .root_cause()
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(libc::ECHILD)
    );
    assert!(!dropped.load(Ordering::Acquire));
    drop(error);
    assert!(dropped.load(Ordering::Acquire));
}

fn authority_loss() -> WaitAuthorityLoss {
    let error = anyhow::Error::new(std::io::Error::from_raw_os_error(libc::ECHILD));
    WaitAuthorityLoss::from_error(&error).unwrap()
}

fn observe_unreaped(authority: &RootWaitAuthority) -> UnreapedRootExit {
    let deadline = Instant::now() + EXIT_PROOF_TIMEOUT;
    loop {
        if let Some(proof) = authority.poll_exited_unreaped().unwrap() {
            return proof;
        }
        assert!(Instant::now() < deadline, "{EXIT_PROOF_TIMEOUT_DETAIL}");
        std::thread::sleep(EXIT_PROOF_POLL_INTERVAL);
    }
}
