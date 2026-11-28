use super::constants::PROCESS_INVENTORY_CAPTURE_ATTEMPTS;
use super::inventory_snapshot::{
    InventoryEntry, install_test_inventory, install_test_inventory_results,
};
use super::inventory_test_support::entry as inventory_entry;
use super::{CapturedProcessTree, DetachedStandardError, ProcessIdentity, spawn_detached};
use std::path::Path;
use std::time::{Duration, Instant};

const SHELL_EXECUTABLE: &str = "/bin/sh";
const SHELL_COMMAND_ARGUMENT: &str = "-c";
const TREE_COMMAND: &str = "/bin/sleep 30 & wait";
const SLEEP_EXECUTABLE: &str = "/bin/sleep";
const SLEEP_DURATION_ARGUMENT: &str = "30";
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);
const DISCOVERY_POLL_INTERVAL: Duration = Duration::from_millis(20);
const ROOT_ONLY_IDENTITY_COUNT: usize = 1;
const STALE_GENERATION_OFFSET: u64 = 1;
const DESCENDANT_MISSING_DETAIL: &str = "descendant was not discovered";
const IDENTITY_SURVIVED_DETAIL: &str = "captured process-tree identity survived SIGKILL";
const EMPTY_INITIAL_REFRESH_DETAIL: &str = "empty initial captured tree was accepted";
const INJECTED_INVENTORY_FAILURE: &str = "injected process inventory failure";

struct TreeFixture {
    child: Option<std::process::Child>,
    tree: CapturedProcessTree,
}

impl TreeFixture {
    fn spawn() -> Self {
        let mut child = spawn_tree();
        let root = ProcessIdentity::capture(child.id()).unwrap();
        let tree = CapturedProcessTree::capture(root).unwrap_or_else(|error| {
            let _ = root.signal(libc::SIGKILL);
            let _ = child.wait();
            panic!("capture test process tree: {error}");
        });
        Self {
            child: Some(child),
            tree,
        }
    }

    fn reap_root(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
    }
}

impl Drop for TreeFixture {
    fn drop(&mut self) {
        let _ = self.tree.terminate(libc::SIGKILL);
        let _ = self.tree.root().signal(libc::SIGKILL);
        self.reap_root();
    }
}

#[test]
fn captured_process_tree_terminates_every_discovered_identity_root_last() {
    let mut fixture = TreeFixture::spawn();
    let identities = wait_for_descendant(&mut fixture.tree);
    let descendants = identities
        .iter()
        .copied()
        .filter(|identity| *identity != fixture.tree.root())
        .collect::<Vec<_>>();

    assert_eq!(identities.last(), Some(&fixture.tree.root()));
    assert_eq!(fixture.tree.live_identities().unwrap(), identities);
    assert_eq!(fixture.tree.live_descendants().unwrap(), descendants);
    let terminated = fixture.tree.terminate(libc::SIGKILL).unwrap();
    assert_eq!(terminated, identities);
    fixture.reap_root();
    wait_for_all_gone(&terminated);
}

#[test]
fn captured_process_tree_rejects_an_empty_initial_refresh() {
    let mut fixture = TreeFixture::spawn();
    let identities = wait_for_descendant(&mut fixture.tree);
    let root = fixture.tree.root();
    let root_entry = entry(root);
    {
        let _inventory =
            install_test_inventory([vec![root_entry], vec![root_entry], Vec::new(), Vec::new()]);
        assert!(
            CapturedProcessTree::capture(root).is_err(),
            "{EMPTY_INITIAL_REFRESH_DETAIL}"
        );
    }
    drop(fixture);
    wait_for_all_gone(&identities);
}

#[test]
fn root_unreaped_proof_survives_a_tracking_fault() {
    let mut fixture = TreeFixture::spawn();
    let proof = fixture.tree.root_exit_proof_for_test(fixture.tree.root());
    fixture.tree.mark_root_exited_unreaped(proof).unwrap();
    {
        let failures =
            std::iter::repeat_with(|| Err(std::io::Error::other(INJECTED_INVENTORY_FAILURE)))
                .take(PROCESS_INVENTORY_CAPTURE_ATTEMPTS);
        let _inventory = install_test_inventory_results(failures);
        assert!(fixture.tree.refresh().is_err());
    }
    let mut victim = spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION_ARGUMENT.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let victim_identity = ProcessIdentity::capture(victim.id()).unwrap();
    let mut victim_entry = entry(victim_identity);
    victim_entry.session_id = fixture.tree.root().pid;
    let refreshed = {
        let _inventory = install_test_inventory([vec![victim_entry], vec![victim_entry]]);
        fixture.tree.refresh().unwrap()
    };

    assert!(refreshed.contains(&victim_identity));
    assert!(victim_identity.matches());
    fixture.tree.terminate(libc::SIGKILL).unwrap();
    victim.wait().unwrap();
}

#[test]
fn root_exit_proof_cannot_authorize_another_process_generation() {
    let mut fixture = TreeFixture::spawn();
    let root = fixture.tree.root();
    let other_generation = ProcessIdentity {
        start_time: root.start_time.wrapping_add(STALE_GENERATION_OFFSET),
        ..root
    };

    let proof = fixture.tree.root_exit_proof_for_test(other_generation);
    let result = fixture.tree.mark_root_exited_unreaped(proof);

    assert!(result.is_err());
}

#[test]
fn retired_tree_never_reclaims_a_later_matching_numeric_session() {
    let mut fixture = TreeFixture::spawn();
    let mut victim = spawn_detached(
        Path::new(SLEEP_EXECUTABLE),
        &[SLEEP_DURATION_ARGUMENT.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let victim_identity = ProcessIdentity::capture(victim.id()).unwrap();
    fixture.tree.retire_session_for_test();
    let mut victim_entry = entry(victim_identity);
    victim_entry.session_id = fixture.tree.root().pid;
    let _inventory = install_test_inventory([vec![victim_entry], vec![victim_entry]]);

    let refreshed = fixture.tree.refresh().unwrap();

    assert!(fixture.tree.session_is_retired());
    assert!(!refreshed.contains(&victim_identity));
    assert!(victim_identity.matches());
    victim_identity.signal(libc::SIGKILL).unwrap();
    victim.wait().unwrap();
}

fn spawn_tree() -> std::process::Child {
    spawn_detached(
        Path::new(SHELL_EXECUTABLE),
        &[SHELL_COMMAND_ARGUMENT.into(), TREE_COMMAND.into()],
        DetachedStandardError::Null,
    )
    .unwrap()
}

fn wait_for_descendant(tree: &mut CapturedProcessTree) -> Vec<ProcessIdentity> {
    let deadline = Instant::now() + DISCOVERY_TIMEOUT;
    loop {
        let identities = tree.refresh().unwrap();
        if identities.len() > ROOT_ONLY_IDENTITY_COUNT {
            return identities;
        }
        assert!(Instant::now() < deadline, "{DESCENDANT_MISSING_DETAIL}");
        std::thread::sleep(DISCOVERY_POLL_INTERVAL);
    }
}

fn wait_for_all_gone(identities: &[ProcessIdentity]) {
    let deadline = Instant::now() + DISCOVERY_TIMEOUT;
    while identities.iter().any(|identity| identity.matches()) {
        assert!(Instant::now() < deadline, "{IDENTITY_SURVIVED_DETAIL}");
        std::thread::sleep(DISCOVERY_POLL_INTERVAL);
    }
}

fn entry(identity: ProcessIdentity) -> InventoryEntry {
    inventory_entry(identity, u32::default(), identity.pid)
}
