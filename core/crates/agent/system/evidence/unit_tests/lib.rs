use super::*;
use std::fs;
use std::os::unix::fs::symlink;
use tempfile::TempDir;

const LAYOUT: EvidenceLayout = EvidenceLayout::new("projects", 4);

fn fixture() -> (TempDir, std::path::PathBuf) {
    let root = TempDir::new().unwrap();
    let directory = root.path().join("projects/encoded/chats");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.jsonl");
    fs::write(&path, "{\"type\":\"user\"}\n").unwrap();
    (root, path)
}

#[test]
fn layout_requires_exact_depth_root_and_normal_components() {
    let root = std::path::Path::new("/tmp/provider-home");
    let ok = root.join("projects/encoded/chats/session.jsonl");
    assert_eq!(
        LAYOUT.relative_names(root, &ok).unwrap().len(),
        4
    );
    assert!(LAYOUT
        .relative_names(root, &root.join("projects/encoded/session.jsonl"))
        .is_none());
    assert!(LAYOUT
        .relative_names(root, &root.join("other/encoded/chats/session.jsonl"))
        .is_none());
    assert!(LAYOUT
        .relative_names(root, &root.join("projects/encoded/../chats/session.jsonl"))
        .is_none());
    assert!(LAYOUT
        .relative_names(std::path::Path::new("relative"), &ok)
        .is_none());
}

#[test]
fn anchored_evidence_opens_verifies_and_reads() {
    let (root, path) = fixture();
    let mut evidence = AnchoredEvidence::open(root.path(), &path, LAYOUT)
        .unwrap()
        .unwrap();
    assert!(evidence.verify().unwrap());
    assert!(evidence.verify_object().unwrap());
    assert!(evidence.durable_object_identity().unwrap().is_some());
    let window = read_anchored(&mut evidence).unwrap();
    assert_eq!(window.values.len(), 1);
    assert!(evidence.verify().unwrap());

    fs::write(&path, "{\"type\":\"user\"}\n{\"type\":\"assistant\"}\n").unwrap();
    assert!(!evidence.verify().unwrap());
    assert!(AnchoredEvidence::open(root.path(), &path, LAYOUT)
        .unwrap()
        .is_some());
}

#[test]
fn symlinked_evidence_is_never_opened() {
    let (root, path) = fixture();
    let link = path.with_file_name("link.jsonl");
    symlink(&path, &link).unwrap();
    assert!(AnchoredEvidence::open(root.path(), &link, LAYOUT)
        .unwrap()
        .is_none());
}

#[test]
fn missing_wrong_depth_and_foreign_roots_return_none() {
    let (root, path) = fixture();
    assert!(
        AnchoredEvidence::open(root.path(), &root.path().join("missing.jsonl"), LAYOUT)
            .unwrap()
            .is_none()
    );
    let other = TempDir::new().unwrap();
    assert!(AnchoredEvidence::open(other.path(), &path, LAYOUT)
        .unwrap()
        .is_none());
    assert!(
        AnchoredEvidence::open(root.path(), &path, EvidenceLayout::new("projects", 5))
            .unwrap()
            .is_none()
    );
}

#[test]
fn reader_parses_complete_lines_and_sees_a_later_append() {
    let (root, path) = fixture();
    fs::write(&path, "{\"a\":1}\n{\"b\":2}\n").unwrap();
    let window = read_with_hook(root.path(), &path, LAYOUT, || {})
        .unwrap()
        .unwrap();
    assert_eq!(window.values.len(), 2);
    assert_eq!(window.head.len(), 2);
    fs::write(&path, "{\"a\":1}\n{\"b\":2}\n{\"c\":3}\n").unwrap();
    let appended = read_with_hook(root.path(), &path, LAYOUT, || {})
        .unwrap()
        .unwrap();
    assert_eq!(appended.values.len(), 3);
}
