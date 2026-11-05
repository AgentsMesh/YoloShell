use crate::constants::MAX_STARTUP_STDERR_BYTES;
use crate::hub::startup_stderr;
use tempfile::tempdir;

#[test]
fn startup_stderr_is_trimmed_and_joined() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("hub-startup.stderr");
    std::fs::write(&path, b"\n first failure \n\n second line\n").unwrap();
    assert_eq!(
        startup_stderr(&path).as_deref(),
        Some("first failure; second line")
    );
}

#[test]
fn startup_stderr_is_bounded() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("hub-startup.stderr");
    std::fs::write(&path, vec![b'x'; MAX_STARTUP_STDERR_BYTES + 64]).unwrap();
    let detail = startup_stderr(&path).unwrap();
    assert_eq!(detail.len(), MAX_STARTUP_STDERR_BYTES);
}

#[test]
fn missing_or_empty_startup_stderr_has_no_detail() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("hub-startup.stderr");
    assert_eq!(startup_stderr(&path), None);
    std::fs::write(&path, b"\n\n").unwrap();
    assert_eq!(startup_stderr(&path), None);
}
