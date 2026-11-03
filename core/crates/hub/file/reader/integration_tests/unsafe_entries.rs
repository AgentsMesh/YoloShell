use crate::fixture::*;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn symlinks_are_rejected_at_the_leaf_and_each_directory_component() {
    let fixture = Fixture::new();
    let root = fixture.root();
    symlink(
        fixture.workspace.join(FILE_PATH),
        fixture.workspace.join(LINK_NAME),
    )
    .unwrap();
    assert_eq!(
        root.read(&fixture.request(LINK_NAME)),
        Err(PublicError::Unauthorized)
    );
    fs::remove_file(fixture.workspace.join(LINK_NAME)).unwrap();
    symlink(
        fixture.workspace.join(SOURCE_DIRECTORY),
        fixture.workspace.join(LINK_NAME),
    )
    .unwrap();
    let relative = format!("{LINK_NAME}/{FILE_NAME}");
    assert_eq!(
        root.read(&fixture.request(&relative)),
        Err(PublicError::Unauthorized)
    );
}

#[test]
fn directories_hardlinks_and_fifos_are_never_treated_as_regular_readable_payloads() {
    let fixture = Fixture::new();
    let root = fixture.root();
    assert_eq!(
        root.read(&fixture.request(SOURCE_DIRECTORY)),
        Err(PublicError::Unauthorized)
    );
    fs::hard_link(
        fixture.workspace.join(FILE_PATH),
        fixture.workspace.join(LINK_NAME),
    )
    .unwrap();
    assert_eq!(
        root.read(&fixture.request(FILE_PATH)),
        Err(PublicError::Unauthorized)
    );
    let name = CString::new(fixture.workspace.join(FIFO_NAME).as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(name.as_ptr(), PRIVATE_MODE as libc::mode_t) },
        0
    );
    assert_eq!(
        root.read(&fixture.request(FIFO_NAME)),
        Err(PublicError::Unauthorized)
    );
}

#[test]
fn a_missing_leaf_is_explicitly_not_found() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.workspace.join(FILE_PATH)).unwrap();
    assert_eq!(
        fixture.root().read(&fixture.request(FILE_PATH)),
        Err(PublicError::NotFound)
    );
}
