use super::*;
use tempfile::tempdir;

const SCAN_TEST_LIMIT: usize = 32;
const SCAN_OVERFLOW_ENTRIES: usize = SCAN_TEST_LIMIT + 1;

#[test]
fn traversal_stops_at_depth_limit_and_never_follows_symlinks() {
    let directory = tempdir().unwrap();
    let mut current = directory.path().to_path_buf();
    for depth in 0..=MAX_EVIDENCE_DEPTH + 1 {
        fs::write(current.join(format!("depth-{depth}.jsonl")), b"{}\n").unwrap();
        let child = current.join("child");
        fs::create_dir(&child).unwrap();
        current = child;
    }
    let files = collect_jsonl(directory.path()).unwrap();
    assert_eq!(files.len(), MAX_EVIDENCE_DEPTH + 1);
    assert!(
        files
            .iter()
            .all(|path| !path.ends_with(format!("depth-{}.jsonl", MAX_EVIDENCE_DEPTH + 1)))
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let target = directory.path().join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("hidden.jsonl"), b"{}\n").unwrap();
        symlink(&target, directory.path().join("linked-directory")).unwrap();
        symlink(
            target.join("hidden.jsonl"),
            directory.path().join("linked-file.jsonl"),
        )
        .unwrap();
        let files = collect_jsonl(directory.path()).unwrap();
        assert!(!files.iter().any(|path| path.ends_with("linked-file.jsonl")));
    }
}

#[test]
fn collection_stops_exactly_at_the_file_limit() {
    let directory = tempdir().unwrap();
    fs::write(directory.path().join("one.jsonl"), b"{}\n").unwrap();
    fs::write(directory.path().join("two.jsonl"), b"{}\n").unwrap();
    assert!(
        collect_jsonl_bounded(
            directory.path(),
            MAX_EVIDENCE_DEPTH,
            0,
            MAX_EVIDENCE_SCAN_ENTRIES,
            MAX_EVIDENCE_PENDING_DIRECTORIES,
        )
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        collect_jsonl_bounded(
            directory.path(),
            MAX_EVIDENCE_DEPTH,
            1,
            MAX_EVIDENCE_SCAN_ENTRIES,
            MAX_EVIDENCE_PENDING_DIRECTORIES,
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn nonmatching_entries_consume_the_total_scan_budget() {
    let directory = tempdir().unwrap();
    for index in 0..SCAN_OVERFLOW_ENTRIES {
        fs::write(directory.path().join(format!("ignored-{index}")), b"keep").unwrap();
    }

    assert!(
        collect_jsonl_bounded(
            directory.path(),
            MAX_EVIDENCE_DEPTH,
            MAX_EVIDENCE_FILES,
            SCAN_TEST_LIMIT,
            MAX_EVIDENCE_PENDING_DIRECTORIES,
        )
        .is_err()
    );
}

#[test]
fn wide_directory_trees_consume_the_pending_budget() {
    let directory = tempdir().unwrap();
    for index in 0..SCAN_OVERFLOW_ENTRIES {
        fs::create_dir(directory.path().join(format!("branch-{index}"))).unwrap();
    }

    assert!(
        collect_jsonl_bounded(
            directory.path(),
            MAX_EVIDENCE_DEPTH,
            MAX_EVIDENCE_FILES,
            MAX_EVIDENCE_SCAN_ENTRIES,
            SCAN_TEST_LIMIT,
        )
        .is_err()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn non_utf8_extensions_are_ignored() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let directory = tempdir().unwrap();
    let name = OsString::from_vec(b"evidence.\xff".to_vec());
    fs::write(directory.path().join(name), b"{}\n").unwrap();
    assert!(collect_jsonl(directory.path()).unwrap().is_empty());
}

#[test]
fn directory_reader_classifies_missing_and_invalid_paths() {
    let directory = tempdir().unwrap();
    assert!(read_directory(directory.path()).unwrap().is_some());
    assert!(
        read_directory(&directory.path().join("missing"))
            .unwrap()
            .is_none()
    );
    let ordinary = directory.path().join("ordinary");
    fs::write(&ordinary, b"data").unwrap();
    assert!(read_directory(&ordinary).is_err());
}

#[cfg(unix)]
#[test]
fn fingerprint_rejects_timestamps_before_the_unix_epoch() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let directory = tempdir().unwrap();
    let path = directory.path().join("historic.jsonl");
    fs::write(&path, b"{}\n").unwrap();
    let name = CString::new(path.as_os_str().as_bytes()).unwrap();
    let times = [
        libc::timespec {
            tv_sec: -1,
            tv_nsec: 0,
        },
        libc::timespec {
            tv_sec: -1,
            tv_nsec: 0,
        },
    ];
    assert_eq!(
        unsafe { libc::utimensat(libc::AT_FDCWD, name.as_ptr(), times.as_ptr(), 0) },
        0
    );
    assert!(fingerprint(&path).is_none());
}

#[test]
fn comparison_handles_one_missing_operand() {
    let directory = tempdir().unwrap();
    let missing = directory.path().join("missing");
    assert!(!same_directory(directory.path(), &missing));
    assert!(!same_directory(&missing, directory.path()));
}
