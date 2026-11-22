use super::delivery_fixture::DeliveryFixture;
use super::delivery_system_constants::*;
use crate::delivery::deliver;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use yolo_release_archive_audit::audit_bytes;
use yolo_release_archive_audit::constants::{
    EXECUTABLE_MODE as ZIP_EXECUTABLE_MODE, UNIX_REGULAR_TYPE,
};
use yolo_release_archive_audit_test_support::{FixtureEntry, build};
use yolo_release_contracts::{
    APP_NAME, CODESIGN_EXECUTABLE, DELIVERY_OUTPUT_FILE_NAMES, DELIVERY_PROCESS_FAILURE_ERROR,
    DELIVERY_SIGNATURE_DESCRIPTION,
};

#[test]
fn system_ditto_and_codesign_deliver_without_launching_the_app() {
    let archive = app_archive(true);
    let fixture = DeliveryFixture::from_archive(archive);
    assert_eq!(deliver(fixture.request()).unwrap(), fixture.expected);
    let mut names = fs::read_dir(&fixture.delivery)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    let mut expected = DELIVERY_OUTPUT_FILE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(names, expected);
}

#[test]
fn system_codesign_failure_is_audited_without_publication() {
    let fixture = DeliveryFixture::from_archive(app_archive(false));
    let error = super::support::error_text(deliver(fixture.request()));
    assert!(error.contains(DELIVERY_PROCESS_FAILURE_ERROR), "{error}");
    assert!(error.contains(DELIVERY_SIGNATURE_DESCRIPTION), "{error}");
    assert!(!fixture.delivery.exists());
    fixture.assert_no_stage();
}

fn app_archive(sign_app: bool) -> Vec<u8> {
    let root = tempfile::tempdir().unwrap();
    let app = root.path().join(APP_NAME);
    let contents = app.join(CONTENTS_DIRECTORY_NAME);
    let macos = contents.join(MACOS_DIRECTORY_NAME);
    fs::create_dir_all(&macos).unwrap();
    let executable = macos.join(TEST_EXECUTABLE_NAME);
    fs::copy(TRUE_EXECUTABLE, &executable).unwrap();
    let info = contents.join(INFO_PLIST_NAME);
    fs::write(&info, INFO_PLIST).unwrap();
    set_mode(&app, FIXTURE_DIRECTORY_MODE);
    set_mode(&contents, FIXTURE_DIRECTORY_MODE);
    set_mode(&macos, FIXTURE_DIRECTORY_MODE);
    set_mode(&executable, FIXTURE_EXECUTABLE_MODE);
    set_mode(&info, FIXTURE_RESOURCE_MODE);
    if sign_app {
        let sign = Command::new(CODESIGN_EXECUTABLE)
            .args(CODESIGN_SIGN_ARGUMENTS)
            .arg(&app)
            .status()
            .unwrap();
        assert!(sign.success());
    }
    normalize_tree(&app, &executable);
    let mut entries = Vec::new();
    append_entries(&app, APP_NAME.to_owned(), &mut entries);
    let archive = build(&entries).bytes;
    audit_bytes(&archive).unwrap();
    archive
}

fn set_mode(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn normalize_tree(path: &Path, executable: &Path) {
    let metadata = fs::symlink_metadata(path).unwrap();
    assert!(!metadata.file_type().is_symlink());
    if metadata.is_dir() {
        set_mode(path, FIXTURE_DIRECTORY_MODE);
        for entry in fs::read_dir(path).unwrap() {
            normalize_tree(&entry.unwrap().path(), executable);
        }
    } else if path == executable {
        set_mode(path, FIXTURE_EXECUTABLE_MODE);
    } else {
        set_mode(path, FIXTURE_RESOURCE_MODE);
    }
}

fn append_entries(path: &Path, archive_path: String, entries: &mut Vec<FixtureEntry>) {
    let metadata = fs::symlink_metadata(path).unwrap();
    if metadata.is_dir() {
        let directory_path = format!("{archive_path}{ARCHIVE_PATH_SEPARATOR}");
        entries.push(FixtureEntry::directory(directory_path.as_bytes()));
        let mut children = fs::read_dir(path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<PathBuf>>();
        children.sort();
        for child in children {
            let name = child.file_name().unwrap().to_str().unwrap();
            append_entries(
                &child,
                format!("{archive_path}{ARCHIVE_PATH_SEPARATOR}{name}"),
                entries,
            );
        }
        return;
    }
    assert!(metadata.is_file());
    let payload = fs::read(path).unwrap();
    let mut entry = FixtureEntry::regular(archive_path.as_bytes());
    entry.payload = payload.clone();
    entry.crc32 = crc32fast::hash(&payload);
    entry.uncompressed_size = u32::try_from(payload.len()).unwrap();
    if metadata.permissions().mode() & yolo_release_contracts::PERMISSION_BITS_MASK
        == FIXTURE_EXECUTABLE_MODE
    {
        entry.unix_mode = UNIX_REGULAR_TYPE | ZIP_EXECUTABLE_MODE;
    }
    entries.push(entry);
}
