use crate::host::tests::*;
use super::*;

const INVALID_MANIFEST_JSON: &[u8] = b"{not-json";
pub(super) const JSON_WHITESPACE_GROWTH: &[u8] = b" ";
pub(super) const MANIFEST_FILE_NAME: &str = "manifest.json";
pub(super) const DURABLE_CHANGED_MESSAGE: &str = "durable state changed while reading";
pub(super) const PRIVATE_MODE_ERROR_MESSAGE: &str = "permissions are not private";
#[cfg(unix)]
pub(super) const PRIVATE_MANIFEST_MODE: u32 = 0o600;
#[cfg(unix)]
const UNREADABLE_MANIFEST_MODE: u32 = 0o000;

#[test]
fn noncanonical_uuid_directory_is_ignored_without_repeated_rebuilds() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let paths = Paths::new(directory.path());
    let alias = paths
        .sessions_dir()
        .join(manifest.terminal_id.simple().to_string());
    std::fs::create_dir(&alias).unwrap();
    atomic_write_json(&alias.join(MANIFEST_FILE_NAME), &manifest).unwrap();
    std::fs::write(paths.catalog(), CORRUPT_CATALOG_BYTES).unwrap();

    let first = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(lock_recover(&first.catalog).list(true).unwrap().len(), 1);
    drop(first);
    let quarantines = catalog_quarantine_count(directory.path());
    let second = Host::open(HostConfig::new(directory.path())).unwrap();
    assert_eq!(lock_recover(&second.catalog).list(true).unwrap().len(), 1);
    drop(second);

    assert_eq!(catalog_quarantine_count(directory.path()), quarantines);
}

#[cfg(unix)]
#[test]
fn malformed_trusted_manifest_fails_closed_without_quarantining_catalog() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    std::fs::write(paths.manifest(terminal_id), INVALID_MANIFEST_JSON).unwrap();
    std::fs::set_permissions(
        paths.manifest(terminal_id),
        std::fs::Permissions::from_mode(PRIVATE_MANIFEST_MODE),
    )
    .unwrap();
    std::fs::write(paths.catalog(), CORRUPT_CATALOG_BYTES).unwrap();

    let error = Host::open(HostConfig::new(directory.path())).err().unwrap();

    assert!(format!("{error:#}").contains("parse durable session manifest"));
    assert_eq!(
        std::fs::read(paths.catalog()).unwrap(),
        CORRUPT_CATALOG_BYTES
    );
    assert_eq!(catalog_quarantine_count(directory.path()), 0);
}

#[cfg(unix)]
#[test]
fn unreadable_trusted_manifest_fails_closed_without_quarantining_catalog() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let paths = Paths::new(directory.path());
    let manifest_path = paths.manifest(manifest.terminal_id);
    std::fs::set_permissions(
        &manifest_path,
        std::fs::Permissions::from_mode(UNREADABLE_MANIFEST_MODE),
    )
    .unwrap();
    std::fs::write(paths.catalog(), CORRUPT_CATALOG_BYTES).unwrap();

    let result = Host::open(HostConfig::new(directory.path()));
    std::fs::set_permissions(
        &manifest_path,
        std::fs::Permissions::from_mode(PRIVATE_MANIFEST_MODE),
    )
    .unwrap();

    let error = result.err().unwrap();
    assert!(format!("{error:#}").contains("open durable session manifest"));
    assert_eq!(
        std::fs::read(paths.catalog()).unwrap(),
        CORRUPT_CATALOG_BYTES
    );
}

#[cfg(unix)]
#[test]
fn manifest_path_replacement_is_rejected_after_descriptor_read() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let original = directory.path().join("original-manifest.json");
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let loaded = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| {
            std::fs::rename(&path, &original).unwrap();
            std::fs::copy(&original, &path).unwrap();
            Ok(())
        },
        |_| Ok(()),
    )
    ;

    assert!(loaded.is_err());
    assert!(original.is_file());
}

#[test]
fn oversized_durable_manifest_is_rejected_before_parsing() {
    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let oversized = catalog_durable_file::MAX_DURABLE_JSON_BYTES as usize + 1;
    std::fs::write(&path, vec![b' '; oversized]).unwrap();
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |_| Ok(()),
        |_| Ok(()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("exceeds byte limit"));
}

#[test]
fn durable_manifest_growth_after_metadata_is_rejected() {
    use std::io::Write;

    let directory = tempdir().unwrap();
    let manifest = persist_rebuild_manifest(directory.path());
    let path = Paths::new(directory.path()).manifest(manifest.terminal_id);
    let original_length = std::fs::metadata(&path).unwrap().len();
    let durable_directory =
        catalog_durable_dir::DurableDirectory::open(path.parent().unwrap()).unwrap();

    let error = catalog_durable_file::read_manifest_with_hooks(
        &durable_directory,
        path.file_name().unwrap(),
        |opened_path| {
            let mut file = std::fs::OpenOptions::new().append(true).open(opened_path)?;
            file.write_all(JSON_WHITESPACE_GROWTH)?;
            Ok(())
        },
        |opened_path| {
            std::fs::OpenOptions::new()
                .write(true)
                .open(opened_path)?
                .set_len(original_length)?;
            Ok(())
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains(DURABLE_CHANGED_MESSAGE));
}

pub(super) fn catalog_quarantine_count(home: &Path) -> usize {
    std::fs::read_dir(home)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(QUARANTINE_PREFIX)
        })
        .count()
}
