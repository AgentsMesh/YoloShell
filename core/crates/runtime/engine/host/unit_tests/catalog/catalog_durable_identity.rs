use crate::host::tests::*;
use super::*;

#[cfg(unix)]
pub(super) const PUBLIC_DURABLE_FILE_MODE: u32 = 0o644;
#[cfg(unix)]
const DURABLE_HARD_LINK_NAME: &str = "manifest-hard-link.json";

#[cfg(unix)]
#[test]
fn durable_manifest_requires_private_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    std::fs::set_permissions(
        &path,
        std::fs::Permissions::from_mode(PUBLIC_DURABLE_FILE_MODE),
    )
    .unwrap();
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |_| Ok(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains(PRIVATE_MODE_ERROR_MESSAGE));
}

#[test]
fn durable_manifest_growth_after_reader_eof_is_rejected() {
    use std::io::Write;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |opened_path| {
            let mut file = std::fs::OpenOptions::new().append(true).open(opened_path)?;
            file.write_all(JSON_WHITESPACE_GROWTH)?;
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_CHANGED_MESSAGE));
}

#[test]
fn durable_manifest_same_length_rewrite_after_reader_eof_is_rejected() {
    use std::io::Write;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |opened_path| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(opened_path)?
                .write_all(b"[")?;
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_CHANGED_MESSAGE));
}

#[cfg(unix)]
#[test]
fn durable_manifest_permission_change_after_read_is_rejected() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |opened_path| {
            std::fs::set_permissions(
                opened_path,
                std::fs::Permissions::from_mode(PUBLIC_DURABLE_FILE_MODE),
            )?;
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_CHANGED_MESSAGE));
}

#[cfg(unix)]
#[test]
fn durable_manifest_link_count_change_after_read_is_rejected() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let hard_link = directory.path().join(DURABLE_HARD_LINK_NAME);
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |opened_path| {
            std::fs::hard_link(opened_path, &hard_link)?;
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_CHANGED_MESSAGE));
}

#[cfg(unix)]
#[test]
fn durable_manifest_fifo_is_rejected_without_waiting_for_a_writer() {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    std::fs::remove_file(&path).unwrap();
    let native_path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe {
            libc::mkfifo(
                native_path.as_ptr(),
                libc::mode_t::try_from(PRIVATE_MANIFEST_MODE).unwrap(),
            )
        },
        0
    );
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_optional_durable_json::<TerminalManifest>(
        &durable_directory,
        path.file_name().unwrap(),
        "session manifest",
    )
    .unwrap_err();

    assert!(error.to_string().contains("is not a regular file"));
}
