use super::*;
use crate::process::inventory_stable::StableInventory;
use crate::process::{
    LostRootWaitAuthority, ProcessIdentity, RootWaitAuthority, WaitAuthorityLoss, wait_direct_child,
};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION: &str = "30";
const EXIT_PROOF_TIMEOUT: Duration = Duration::from_secs(5);
const EXIT_PROOF_POLL_INTERVAL: Duration = Duration::from_millis(10);
const EXIT_PROOF_TIMEOUT_DETAIL: &str = "waitid did not observe the exact root exit";
const ROOT_GENERATION_INCREMENT: u64 = 1;

struct Fixture {
    root: ProcessIdentity,
    tree: CapturedProcessTree,
    lost: Option<LostRootWaitAuthority>,
    stale_unreaped: Option<UnreapedRootExit>,
}

impl Fixture {
    fn lost_while_live() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
        );
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(root).unwrap();
        let child = Box::new(child) as crate::process::OwnedPtyChild;
        let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
            .unwrap()
            .bind(child);
        let loss = anyhow::Error::new(std::io::Error::from_raw_os_error(libc::ECHILD));
        let lost = authority.lose(WaitAuthorityLoss::from_error(&loss).unwrap());
        Self {
            root,
            tree,
            lost: Some(lost),
            stale_unreaped: None,
        }
    }

    fn externally_reaped() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
        );
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(root).unwrap();
        let child = Box::new(child) as crate::process::OwnedPtyChild;
        let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
            .unwrap()
            .bind(child);
        assert!(root.signal(libc::SIGKILL).unwrap());
        wait_direct_child(root.pid).unwrap();
        let loss = anyhow::Error::new(std::io::Error::from_raw_os_error(libc::ECHILD));
        let lost = authority.lose(WaitAuthorityLoss::from_error(&loss).unwrap());
        Self {
            root,
            tree,
            lost: Some(lost),
            stale_unreaped: None,
        }
    }

    fn proof(&mut self) -> RootGoneWithoutWait {
        self.lost.as_ref().unwrap().observe_gone().unwrap().unwrap()
    }

    fn reaped_after_unreaped_proof() -> Self {
        let child = crate::process::spawn_waitable_test_child(
            Command::new(Path::new(SLEEP_EXECUTABLE)).arg(SLEEP_DURATION),
        );
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let mut tree = CapturedProcessTree::capture(root).unwrap();
        let child = Box::new(child) as crate::process::OwnedPtyChild;
        let authority = RootWaitAuthority::prove_owned_child(&child, &tree)
            .unwrap()
            .bind(child);
        assert!(root.signal(libc::SIGKILL).unwrap());
        let unreaped = observe_unreaped(&authority);
        tree.mark_root_exited_unreaped(unreaped).unwrap();
        wait_direct_child(root.pid).unwrap();
        let loss = anyhow::Error::new(std::io::Error::from_raw_os_error(libc::ECHILD));
        let lost = authority.lose(WaitAuthorityLoss::from_error(&loss).unwrap());
        Self {
            root,
            tree,
            lost: Some(lost),
            stale_unreaped: Some(unreaped),
        }
    }
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

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.root.signal(libc::SIGKILL);
        let _ = wait_direct_child(self.root.pid);
    }
}

#[test]
fn lost_wait_authority_keeps_live_root_witness() {
    let fixture = Fixture::lost_while_live();

    assert!(
        fixture
            .lost
            .as_ref()
            .unwrap()
            .observe_gone()
            .unwrap()
            .is_none()
    );
    assert_eq!(
        fixture.tree.root_ownership_proof(),
        SessionOwnershipProof::LiveWitness
    );
}

#[test]
fn gone_without_wait_enables_capability_but_disables_numeric_session_scan() {
    let mut fixture = Fixture::externally_reaped();
    let proof = fixture.proof();
    fixture.tree.mark_root_gone_without_wait(proof).unwrap();
    let ownership = fixture.tree.root_ownership_proof();
    let inventory = StableInventory::capture().unwrap();

    assert!(ownership.enables_capability_scan());
    assert_eq!(
        fixture
            .tree
            .scope
            .live_session_id(&inventory, &[], ownership),
        None
    );
}

#[test]
fn gone_without_wait_strengthens_unreaped_root_state() {
    let mut fixture = Fixture::reaped_after_unreaped_proof();
    let proof = fixture.proof();

    fixture.tree.mark_root_gone_without_wait(proof).unwrap();
    fixture
        .tree
        .mark_root_exited_unreaped(fixture.stale_unreaped.take().unwrap())
        .unwrap();

    assert_eq!(
        fixture.tree.root_ownership_proof(),
        SessionOwnershipProof::RootGoneWithoutWait
    );
}

#[test]
fn retired_session_absorbs_only_its_exact_late_root_exit_proof() {
    let mut fixture = Fixture::lost_while_live();
    let proof = fixture.tree.root_exit_proof_for_test(fixture.root);
    fixture.tree.retire_session_for_test();

    fixture.tree.mark_root_exited_unreaped(proof).unwrap();

    assert!(fixture.tree.session_is_retired());
    assert_eq!(
        fixture.tree.root_ownership_proof(),
        SessionOwnershipProof::SessionRetired
    );
    let other = ProcessIdentity {
        start_time: fixture
            .root
            .start_time
            .wrapping_add(ROOT_GENERATION_INCREMENT),
        ..fixture.root
    };
    let stale = fixture.tree.root_exit_proof_for_test(other);
    assert!(fixture.tree.mark_root_exited_unreaped(stale).is_err());
}
