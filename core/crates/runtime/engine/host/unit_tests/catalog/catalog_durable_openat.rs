use crate::host::tests::*;
use super::*;

#[cfg(unix)]
#[test]
fn sessions_openat_rejects_a_root_replaced_with_a_symlink() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    paths.ensure_base().unwrap();
    let home = catalog_durable_dir::DurableDirectory::open(paths.home()).unwrap();
    let displaced = directory.path().join("displaced-sessions");
    let outside = directory.path().join("outside-sessions");
    std::fs::create_dir(&outside).unwrap();
    std::fs::rename(paths.sessions_dir(), &displaced).unwrap();
    symlink(&outside, paths.sessions_dir()).unwrap();

    let opened = home.open_directory(paths.sessions_dir().file_name().unwrap());

    assert!(opened.is_err());
}

#[cfg(unix)]
#[test]
fn session_openat_rejects_a_uuid_directory_replaced_with_a_symlink() {
    use std::os::unix::fs::symlink;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let paths = Paths::new(directory.path());
    let home = catalog_durable_dir::DurableDirectory::open(paths.home()).unwrap();
    let sessions = home
        .open_directory(paths.sessions_dir().file_name().unwrap())
        .unwrap()
        .unwrap();
    let session_path = paths.session_dir(manifest.terminal_id);
    let displaced = directory.path().join("displaced-session");
    let outside = directory.path().join("outside-session");
    std::fs::create_dir(&outside).unwrap();
    std::fs::rename(&session_path, &displaced).unwrap();
    symlink(&outside, &session_path).unwrap();

    let opened = sessions.open_directory(session_path.file_name().unwrap());

    assert!(opened.is_err());
}

#[cfg(unix)]
#[test]
fn opened_uuid_descriptor_rejects_public_path_replacement() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let paths = Paths::new(directory.path());
    let session_path = paths.session_dir(manifest.terminal_id);
    let home = catalog_durable_dir::DurableDirectory::open(paths.home()).unwrap();
    let sessions = home
        .open_directory(paths.sessions_dir().file_name().unwrap())
        .unwrap()
        .unwrap();
    let session = sessions
        .open_directory(session_path.file_name().unwrap())
        .unwrap()
        .unwrap();
    let displaced = directory.path().join("opened-session");
    std::fs::rename(&session_path, &displaced).unwrap();
    std::fs::create_dir(&session_path).unwrap();
    let manifest_path = paths.manifest(manifest.terminal_id);

    let loaded = catalog_durable_file::read_optional_durable_json::<TerminalManifest>(
        &session,
        manifest_path.file_name().unwrap(),
        "openat session manifest",
    );

    assert!(loaded.is_err());
    assert!(displaced.join(MANIFEST_FILE_NAME).is_file());
}
