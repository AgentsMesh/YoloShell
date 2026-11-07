use super::super::test_support::ProbeFixture;
use super::super::{
    ENTRY_SET_CHANGED, ENTRY_VERSION_CHANGED, ObservationBoundary, read_exact_session,
    read_exact_session_observing,
};
use super::*;
use crate::paths::constants::PRIVATE_FILE_MODE;
use crate::paths::test_support::retired_storage_artifact_name;
use std::ffi::{CString, OsStr};
use std::fs::{File, OpenOptions, Permissions};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};

const UNBOUND_TARGET_NAME: &str = "unbound-guardian-probe.json";
const HARD_LINK_SOURCE_NAME: &str = "storage-artifact-hard-link-source";
const REPLACED_ARTIFACT_NAME: &str = "storage-artifact-replaced";
const UNSAFE_FILE_MODE: u32 = 0o640;
const UNEXPECTED_OBSERVATION_DETAIL: &str = "unexpected observation failure";
#[cfg(target_os = "macos")]
const SOCKET_ALIAS_PARENT: &str = "/private/tmp";
#[cfg(not(target_os = "macos"))]
const SOCKET_ALIAS_PARENT: &str = "/tmp";
const SOCKET_ALIAS_PREFIX: &str = "yolo-probe-socket-";
const SOCKET_PARENT_ALIAS: &str = "parent";

#[test]
fn exact_private_storage_artifact_is_ignored() {
    let fixture = ProbeFixture::new();
    let artifact = exact_artifact_path(&fixture);
    create_private_file(&artifact).unwrap();

    assert!(read_fixture(&fixture).is_ok());
}

#[test]
fn storage_artifact_without_an_exact_target_is_rejected() {
    let fixture = ProbeFixture::new();
    let target = current_probe_path(&fixture).with_file_name(UNBOUND_TARGET_NAME);
    let artifact = retired_artifact_path(&target);
    create_private_file(&artifact).unwrap();

    assert_rejected(&fixture, STORAGE_ARTIFACT_BINDING_DETAIL);
}

#[test]
fn storage_artifact_directory_is_rejected() {
    let fixture = ProbeFixture::new();
    std::fs::create_dir(exact_artifact_path(&fixture)).unwrap();

    assert_rejected(&fixture, STORAGE_ARTIFACT_OPEN_DETAIL);
}

#[test]
fn storage_artifact_symlink_is_rejected() {
    let fixture = ProbeFixture::new();
    let target = current_probe_path(&fixture);
    symlink(&target, exact_artifact_path(&fixture)).unwrap();

    assert_rejected(&fixture, STORAGE_ARTIFACT_OPEN_DETAIL);
}

#[test]
fn storage_artifact_unix_socket_is_rejected() {
    let fixture = ProbeFixture::new();
    let artifact = exact_artifact_path(&fixture);
    let (_alias, _socket) = bind_socket_at(&artifact).unwrap();

    assert_rejected(&fixture, STORAGE_ARTIFACT_OPEN_DETAIL);
}

#[test]
fn storage_artifact_with_unsafe_mode_is_rejected() {
    let fixture = ProbeFixture::new();
    let artifact = exact_artifact_path(&fixture);
    create_private_file(&artifact).unwrap();
    std::fs::set_permissions(&artifact, Permissions::from_mode(UNSAFE_FILE_MODE)).unwrap();

    assert_rejected(&fixture, STORAGE_ARTIFACT_OPEN_DETAIL);
}

#[test]
fn hard_linked_storage_artifact_is_rejected() {
    let fixture = ProbeFixture::new();
    let source = session_directory(&fixture).join(HARD_LINK_SOURCE_NAME);
    create_private_file(&source).unwrap();
    std::fs::hard_link(source, exact_artifact_path(&fixture)).unwrap();

    assert_rejected(&fixture, STORAGE_ARTIFACT_OPEN_DETAIL);
}

#[test]
fn storage_artifact_inode_replacement_is_rejected() {
    let fixture = ProbeFixture::new();
    let artifact = exact_artifact_path(&fixture);
    let replaced = session_directory(&fixture).join(REPLACED_ARTIFACT_NAME);
    create_private_file(&artifact).unwrap();

    let result = read_exact_session_observing(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
        |boundary| {
            if boundary == ObservationBoundary::BeforeStableVerification {
                std::fs::rename(&artifact, &replaced)?;
                create_private_file(&artifact)?;
            }
            Ok(())
        },
    );

    assert_error_contains(result.unwrap_err(), ENTRY_VERSION_CHANGED);
}

#[test]
fn storage_artifact_inserted_between_scans_changes_the_entry_set() {
    let fixture = ProbeFixture::new();
    let artifact = exact_artifact_path(&fixture);

    let result = read_exact_session_observing(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
        |boundary| {
            if boundary == ObservationBoundary::BeforeSecondScan {
                create_private_file(&artifact)?;
            }
            Ok(())
        },
    );

    assert_error_contains(result.unwrap_err(), ENTRY_SET_CHANGED);
}

fn read_fixture(fixture: &ProbeFixture) -> anyhow::Result<crate::process::PtyMasterFdCustodyProbe> {
    read_exact_session(
        &fixture.namespace,
        &fixture.paths,
        &fixture.record,
        fixture.identity,
    )
}

fn assert_rejected(fixture: &ProbeFixture, detail: &str) {
    assert_error_contains(read_fixture(fixture).unwrap_err(), detail);
}

fn assert_error_contains(error: anyhow::Error, detail: &str) {
    assert!(
        format!("{error:#}").contains(detail),
        "{UNEXPECTED_OBSERVATION_DETAIL}: {error:#}"
    );
}

fn exact_artifact_path(fixture: &ProbeFixture) -> PathBuf {
    retired_artifact_path(&current_probe_path(fixture))
}

fn retired_artifact_path(target: &Path) -> PathBuf {
    let target_name = CString::new(target.file_name().unwrap().as_bytes()).unwrap();
    let artifact = retired_storage_artifact_name(&target_name).unwrap();
    target.with_file_name(OsStr::from_bytes(artifact.as_bytes()))
}

fn current_probe_path(fixture: &ProbeFixture) -> PathBuf {
    let guardian = fixture.record.armed_lease().unwrap().guardian.unwrap();
    super::super::super::path(
        &fixture.paths,
        fixture.record.runtime.terminal_id,
        guardian.guardian_id(),
    )
}

fn session_directory(fixture: &ProbeFixture) -> PathBuf {
    fixture.paths.session_dir(fixture.record.runtime.terminal_id)
}

fn create_private_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
}

fn bind_socket_at(path: &Path) -> std::io::Result<(tempfile::TempDir, UnixListener)> {
    let root = tempfile::Builder::new()
        .prefix(SOCKET_ALIAS_PREFIX)
        .tempdir_in(SOCKET_ALIAS_PARENT)?;
    let alias = root.path().join(SOCKET_PARENT_ALIAS);
    symlink(path.parent().unwrap(), &alias)?;
    let listener = UnixListener::bind(alias.join(path.file_name().unwrap()))?;
    Ok((root, listener))
}
