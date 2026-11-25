use super::gc_tests::candidate_path;
use super::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use std::path::{Path, PathBuf};
use yolo_maintenance_contract::ScanChunk;

const CHUNK_ENTRIES: usize = 4;
const CHUNK_DELETIONS: usize = 2;
const UNKNOWN_ENTRIES: usize = CHUNK_ENTRIES * 3;
const STALE_ENTRIES: usize = CHUNK_ENTRIES * 3;
const MAX_TEST_CHUNKS: usize = 256;

fn root() -> (tempfile::TempDir, PrivateDirectory) {
    let temp = tempfile::tempdir().unwrap();
    let root = PrivateDirectory::ensure(&temp.path().join("coordination")).unwrap();
    (temp, root)
}

fn stale(root: &Path, index: usize) -> PathBuf {
    let path = candidate_path(root, index);
    drop(
        PrivateFileLock::try_acquire(&path, MAX_COORDINATION_LOCK_BYTES)
            .unwrap()
            .unwrap(),
    );
    path
}

#[test]
fn descriptor_revision_restarts_a_mutated_pass() {
    let (_temp, root) = root();
    for index in 0..UNKNOWN_ENTRIES {
        std::fs::write(root.path().join(format!("unknown-{index}")), b"value").unwrap();
    }
    let mut scanner = scavenge::CoordinationScanner::from_root(root.try_clone().unwrap());
    let (progress, _) = scanner
        .scan_with_limits(CHUNK_ENTRIES, CHUNK_DELETIONS)
        .unwrap();
    assert_eq!(progress, ScanChunk::Progress);
    let candidate = stale(root.path(), UNKNOWN_ENTRIES);
    for _ in 0..MAX_TEST_CHUNKS {
        let (progress, _) = scanner
            .scan_with_limits(CHUNK_ENTRIES, CHUNK_DELETIONS)
            .unwrap();
        if progress == ScanChunk::PassComplete {
            break;
        }
    }
    assert!(!candidate.exists());
}

#[test]
fn scanner_restart_from_zero_still_converges() {
    let (_temp, root) = root();
    let candidates: Vec<_> = (0..STALE_ENTRIES)
        .map(|index| stale(root.path(), index))
        .collect();
    let mut interrupted = scavenge::CoordinationScanner::from_root(root.try_clone().unwrap());
    interrupted
        .scan_with_limits(CHUNK_ENTRIES, CHUNK_DELETIONS)
        .unwrap();
    drop(interrupted);
    scavenge::sweep_pass_for_test(root.path(), CHUNK_ENTRIES, CHUNK_DELETIONS).unwrap();
    assert!(candidates.iter().all(|path| !path.exists()));
}

#[test]
fn lifecycle_contention_is_nonblocking_and_does_no_work() {
    let (_temp, root) = root();
    let _lifecycle = lifecycle::LifecycleLock::try_acquire(&root)
        .unwrap()
        .unwrap();
    let mut scanner = scavenge::CoordinationScanner::from_root(root.try_clone().unwrap());
    let (progress, work) = scanner
        .scan_with_limits(CHUNK_ENTRIES, CHUNK_DELETIONS)
        .unwrap();
    assert_eq!(progress, ScanChunk::Busy);
    assert_eq!((work.inspected, work.probed, work.deleted), (0, 0, 0));
}
