use crate::host::tests::*;
use super::*;

#[cfg(unix)]
#[test]
fn catalog_rebuild_rejects_a_symlinked_durable_manifest() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.terminal_id = terminal_id;
    let outside = directory.path().join("outside-manifest.json");
    atomic_write_json(&outside, &manifest).unwrap();
    let outside_bytes = std::fs::read(&outside).unwrap();
    symlink(&outside, paths.manifest(terminal_id)).unwrap();
    std::fs::write(paths.catalog(), CORRUPT_CATALOG_BYTES).unwrap();

    let error = Host::open(HostConfig::new(directory.path())).err().unwrap();

    assert!(error.chain().any(|cause| {
        cause
            .downcast_ref::<std::io::Error>()
            .and_then(std::io::Error::raw_os_error)
            == Some(libc::ELOOP)
    }));
    assert_eq!(std::fs::read(outside).unwrap(), outside_bytes);
}
