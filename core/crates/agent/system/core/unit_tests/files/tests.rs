use super::*;
use tempfile::tempdir;

#[test]
fn collection_is_bounded_to_jsonl_files_and_handles_missing_roots() {
    let directory = tempdir().unwrap();
    let nested = directory.path().join("a/b");
    fs::create_dir_all(&nested).unwrap();
    fs::write(directory.path().join("root.jsonl"), b"{}\n").unwrap();
    fs::write(nested.join("nested.jsonl"), b"{}\n").unwrap();
    fs::write(nested.join("ignored.txt"), b"{}\n").unwrap();
    let files = collect_jsonl(directory.path()).unwrap();
    assert_eq!(files.len(), 2);
    assert!(
        collect_jsonl(&directory.path().join("missing"))
            .unwrap()
            .is_empty()
    );

    let ordinary_file = directory.path().join("ordinary");
    fs::write(&ordinary_file, b"data").unwrap();
    assert!(collect_jsonl(&ordinary_file).is_err());
}

#[test]
fn fingerprint_and_directory_comparison_cover_edge_cases() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("short.jsonl");
    fs::write(&path, b"{\"ok\":true}\ninvalid\n{\"value\":2}\n").unwrap();
    let observed = fingerprint(&path).unwrap();
    assert!(observed.len > 0);
    assert!(observed.modified_ms > 0);
    assert!(fingerprint(&directory.path().join("missing")).is_none());
    assert!(same_directory(
        directory.path(),
        &directory.path().join(".")
    ));
    let missing = directory.path().join("missing");
    assert!(same_directory(&missing, &missing));
    assert!(!same_directory(&missing, &directory.path().join("other")));
}
