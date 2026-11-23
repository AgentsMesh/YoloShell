use super::*;
use crate::journal::Journal;

const JOURNAL_SENTINEL_NAME: &str = "sentinel";
const JOURNAL_SENTINEL_VALUE: &str = "replacement";
const REPLACEMENT_MANIFEST_VALUE: &str = "replacement manifest";

#[test]
fn worker_namespace_detects_home_replacement() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let displaced = root.path().join("displaced");
    let paths = Paths::new(&home);
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    std::fs::rename(&home, displaced).unwrap();
    paths.ensure_session(terminal_id).unwrap();
    assert!(namespace.verify().is_err());
}

#[test]
fn retained_journal_open_never_creates_in_replacement() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let displaced = root.path().join("displaced");
    let paths = Paths::new(&home);
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let journal = paths.journal(terminal_id);
    let entry = namespace.file(&journal).unwrap();
    std::fs::rename(&home, &displaced).unwrap();
    paths.ensure_session(terminal_id).unwrap();
    let sentinel = home.join(JOURNAL_SENTINEL_NAME);
    std::fs::write(&sentinel, JOURNAL_SENTINEL_VALUE).unwrap();

    assert!(Journal::open_retained(entry, Uuid::new_v4()).is_err());
    assert!(!journal.exists());
    assert_eq!(
        std::fs::read_to_string(sentinel).unwrap(),
        JOURNAL_SENTINEL_VALUE
    );
}

#[test]
fn retained_manifest_check_rejects_home_replacement() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let displaced = root.path().join("displaced");
    let paths = Paths::new(&home);
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    std::fs::rename(&home, &displaced).unwrap();
    paths.ensure_session(terminal_id).unwrap();
    let manifest = paths.manifest(terminal_id);
    std::fs::write(&manifest, REPLACEMENT_MANIFEST_VALUE).unwrap();

    assert!(namespace.file_is_missing(&manifest).is_err());
    assert_eq!(
        std::fs::read_to_string(manifest).unwrap(),
        REPLACEMENT_MANIFEST_VALUE
    );
}

#[test]
fn worker_namespace_detects_socket_parent_replacement() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::new(root.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let socket_parent = paths.socket_dir().unwrap();
    let displaced = socket_parent.with_extension("displaced");
    std::fs::rename(&socket_parent, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&socket_parent).unwrap();
    assert!(namespace.verify().is_err());
    std::fs::remove_dir_all(socket_parent).unwrap();
    std::fs::remove_dir_all(displaced).unwrap();
}
