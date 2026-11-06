use super::*;
use std::fs::File;

#[test]
fn c_paths_require_termination_and_content() {
    assert_eq!(
        path_from_c_bytes(b"/tmp/evidence.jsonl\0ignored"),
        Some(PathBuf::from("/tmp/evidence.jsonl"))
    );
    assert_eq!(path_from_c_bytes(b"\0ignored"), None);
    assert_eq!(path_from_c_bytes(b"unterminated"), None);
}

#[test]
fn descriptor_inventory_rejects_invalid_identity_and_budget() {
    assert!(paths(0, 1).is_err());
    assert!(paths(u32::MAX, 1).is_err());
    assert!(paths(i32::MAX as u32, 1).is_err());
    assert!(paths(std::process::id(), 0).is_err());
    assert!(paths(std::process::id(), usize::MAX).is_err());
    assert!(paths(std::process::id(), i32::MAX as usize).is_err());
}

#[test]
fn current_process_jsonl_descriptor_is_associated_and_deduplicated() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("evidence.jsonl");
    let first = File::create(&path).unwrap();
    let second = File::open(&path).unwrap();
    let observed = paths(
        std::process::id(),
        crate::agent::constants::MAX_PROCESS_FILE_DESCRIPTORS,
    )
    .unwrap();
    let expected = path.canonicalize().unwrap();
    assert_eq!(
        observed
            .iter()
            .filter_map(|value| value.canonicalize().ok())
            .filter(|value| value == &expected)
            .count(),
        1
    );
    drop((first, second));
}

#[test]
fn evidence_path_inventory_is_bounded() {
    let directory = tempfile::tempdir().unwrap();
    let files = (0..=MAX_PROCESS_EVIDENCE_PATHS)
        .map(|index| File::create(directory.path().join(format!("{index}.jsonl"))).unwrap())
        .collect::<Vec<_>>();
    assert!(
        paths(
            std::process::id(),
            crate::agent::constants::MAX_PROCESS_FILE_DESCRIPTORS,
        )
        .is_err()
    );
    drop(files);
}
