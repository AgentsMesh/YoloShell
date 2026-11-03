use super::*;
use crate::coordination::gc_tests::candidate_path;
use crate::maintenance::ScanChunk;
use crate::paths::private_file_lock::PrivateFileLock;
use std::ffi::OsStr;

const CANDIDATE_INDEX: usize = 73;
const UNSAFE_CANDIDATE_INDEX: usize = CANDIDATE_INDEX + 1;
const EXPECTED_REVISION_FAILURE: &str = "expected injected revision failure";
const INJECTED_REVISION_FAILURE: &str = "injected revision failure";
const LOGICAL_FAILURE: &str = "unproven logical failure";
const STATE_DIRECTORY: &str = "coordination";
const UNKNOWN_ENTRY: &str = "unknown-entry";
const ZERO_DELETE_BUDGET: usize = 0;
const ZERO_ENTRY_BUDGET: usize = 0;

fn root() -> (tempfile::TempDir, PrivateDirectory) {
    let temporary = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::ensure(&temporary.path().join(STATE_DIRECTORY)).unwrap();
    (temporary, root)
}

fn stale_candidate(root: &PrivateDirectory) -> PathBuf {
    let path = candidate_path(root.path(), CANDIDATE_INDEX);
    drop(
        PrivateFileLock::try_acquire(&path, MAX_COORDINATION_LOCK_BYTES)
            .unwrap()
            .unwrap(),
    );
    path
}

#[test]
fn scanner_opens_an_uninitialized_root_and_respects_a_zero_delete_budget() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join(STATE_DIRECTORY);
    let mut scanner = CoordinationScanner::from_path(path.clone());
    let (_, empty) = scanner
        .scan_with_limits(SWEEP_ENTRY_BUDGET, ZERO_DELETE_BUDGET)
        .unwrap();
    assert!(path.is_dir());
    assert_eq!(empty.deleted, 0);

    let root = PrivateDirectory::open(&path).unwrap();
    let candidate = stale_candidate(&root);
    let (_, work) = scanner
        .scan_with_limits(SWEEP_ENTRY_BUDGET, ZERO_DELETE_BUDGET)
        .unwrap();
    assert_eq!(work.deleted, 0);
    assert!(candidate.exists());
}

#[test]
fn revision_failure_resets_the_scanner_and_a_later_pass_recovers() {
    let (_temporary, root) = root();
    let path = root.path().to_path_buf();
    let mut scanner = CoordinationScanner::from_root(root);
    let error = scanner
        .scan_with_limits_using(SWEEP_ENTRY_BUDGET, SWEEP_DELETE_BUDGET, |_| {
            anyhow::bail!(INJECTED_REVISION_FAILURE)
        })
        .err()
        .expect(EXPECTED_REVISION_FAILURE);
    assert!(error.to_string().contains(INJECTED_REVISION_FAILURE));
    assert!(scanner.root.is_none());
    assert!(
        scanner
            .scan_with_limits(ZERO_ENTRY_BUDGET, SWEEP_DELETE_BUDGET)
            .is_ok()
    );
    assert_eq!(scanner.root.as_ref().unwrap().path(), path);
}

#[test]
fn an_external_revision_change_restarts_enumeration_without_deletion() {
    let (_temporary, root) = root();
    let mut scanner = CoordinationScanner::from_root(root);
    let (result, work) = scanner
        .scan_with_limits_using(ZERO_ENTRY_BUDGET, SWEEP_DELETE_BUDGET, |directory| {
            std::fs::write(directory.path().join(UNKNOWN_ENTRY), [])?;
            directory.revision()
        })
        .unwrap();

    assert_eq!(result, ScanChunk::Progress);
    assert_eq!(work.deleted, 0);
    assert!(scanner.stream.is_none());
}

#[test]
fn candidate_name_and_live_lease_failures_are_non_destructive() {
    assert!(!candidate::is_lock_name(OsStr::new("worker-.lock")));
    assert!(candidate::changed_or_unproven(&anyhow::anyhow!(
        LOGICAL_FAILURE
    )));
    let (_temporary, root) = root();
    let candidate = stale_candidate(&root);
    let lease = CoordinationLease::try_acquire(&candidate).unwrap().unwrap();
    #[cfg(target_os = "linux")]
    let _file_lock = crate::paths::private_file_lock::PrivateFileLock::try_acquire(
        &candidate,
        MAX_COORDINATION_LOCK_BYTES,
    )
    .unwrap()
    .unwrap();

    assert!(!candidate::clean(&root, candidate.file_name().unwrap()).unwrap());
    assert!(candidate.exists());
    drop(lease);

    let unsafe_candidate = candidate_path(root.path(), UNSAFE_CANDIDATE_INDEX);
    std::fs::create_dir(&unsafe_candidate).unwrap();
    assert!(!candidate::clean(&root, unsafe_candidate.file_name().unwrap()).unwrap());
    assert!(unsafe_candidate.is_dir());
}
