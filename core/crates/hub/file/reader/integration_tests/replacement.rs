use crate::fixture::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn retained_scope_cannot_adopt_a_replacement_workspace() {
    let fixture = Fixture::new();
    let root = fixture.root();
    fs::rename(&fixture.workspace, fixture.base.join(SAVED_NAME)).unwrap();
    fs::create_dir_all(fixture.workspace.join(SOURCE_DIRECTORY)).unwrap();
    fs::write(fixture.workspace.join(FILE_PATH), PAYLOAD).unwrap();
    assert_eq!(
        root.read(&fixture.request(FILE_PATH)),
        Err(PublicError::Conflict)
    );
}

#[test]
fn prepared_file_rejects_replacement_or_reparenting_before_reading() {
    for replace_directory in [false, true] {
        let fixture = Fixture::new();
        let prepared = fixture.root().prepare(&fixture.request(FILE_PATH)).unwrap();
        if replace_directory {
            fs::rename(
                fixture.workspace.join(SOURCE_DIRECTORY),
                fixture.workspace.join(SAVED_NAME),
            )
            .unwrap();
            fs::create_dir(fixture.workspace.join(SOURCE_DIRECTORY)).unwrap();
        } else {
            fs::rename(
                fixture.workspace.join(FILE_PATH),
                fixture.workspace.join(SAVED_NAME),
            )
            .unwrap();
        }
        fs::write(fixture.workspace.join(FILE_PATH), PAYLOAD).unwrap();
        assert_eq!(prepared.read(), Err(PublicError::Conflict));
    }
}

#[test]
fn prepared_file_rejects_mutation_truncation_and_permission_changes() {
    for mutation in [Mutation::Content, Mutation::Truncate, Mutation::Permissions] {
        let fixture = Fixture::new();
        let path = fixture.workspace.join(FILE_PATH);
        fs::set_permissions(&path, fs::Permissions::from_mode(PROJECT_MODE)).unwrap();
        let prepared = fixture.root().prepare(&fixture.request(FILE_PATH)).unwrap();
        match mutation {
            Mutation::Content => fs::write(&path, CHANGED).unwrap(),
            Mutation::Truncate => fs::write(&path, []).unwrap(),
            Mutation::Permissions => {
                fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_MODE)).unwrap()
            }
        }
        assert_eq!(prepared.read(), Err(PublicError::Conflict));
    }
}

#[test]
fn loss_of_a_protected_namespace_retires_a_prepared_read() {
    let fixture = Fixture::new();
    let prepared = fixture.root().prepare(&fixture.request(FILE_PATH)).unwrap();
    fs::rename(&fixture.private, fixture.base.join(SAVED_NAME)).unwrap();
    fs::create_dir(&fixture.private).unwrap();
    assert_eq!(prepared.read(), Err(PublicError::Conflict));
}

enum Mutation {
    Content,
    Truncate,
    Permissions,
}
