use crate::fixture::*;
use std::fs;
use std::os::unix::fs::symlink;
use yolo_hub_file_protocol::FileRootSummary;
use yolo_hub_file_reader::ReadRoot;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn scopes_cannot_contain_or_be_inside_a_protected_service_directory() {
    let fixture = Fixture::new();
    let nested = fixture.private.join(WORKSPACE_DIRECTORY);
    fs::create_dir(&nested).unwrap();
    for path in [&fixture.private, &fixture.base, &nested] {
        let mut summary = fixture.summary.fields().clone();
        summary.path = path.to_str().unwrap().into();
        assert!(matches!(
            ReadRoot::open(
                FileRootSummary::new(summary).unwrap(),
                &[fixture.private.clone()]
            ),
            Err(PublicError::Unauthorized)
        ));
    }
    assert!(matches!(
        ReadRoot::open(fixture.summary.clone(), &[]),
        Err(PublicError::Unauthorized)
    ));
}

#[test]
fn protected_aliases_are_compared_by_object_identity_not_path_spelling() {
    let fixture = Fixture::new();
    let alias = fixture.base.join(LINK_NAME);
    symlink(&fixture.workspace, &alias).unwrap();
    assert!(matches!(
        ReadRoot::open(fixture.summary.clone(), &[alias]),
        Err(PublicError::Unauthorized)
    ));
    let mut summary = fixture.summary.fields().clone();
    summary.path = fixture.base.join(LINK_NAME).to_str().unwrap().into();
    assert!(matches!(
        ReadRoot::open(
            FileRootSummary::new(summary).unwrap(),
            &[fixture.private.clone()]
        ),
        Err(PublicError::Unauthorized)
    ));
}

#[test]
fn missing_protected_paths_fail_closed_instead_of_weakening_the_exclusion() {
    let fixture = Fixture::new();
    fs::remove_dir(&fixture.private).unwrap();
    assert!(matches!(
        ReadRoot::open(fixture.summary, &[fixture.private]),
        Err(PublicError::NotFound)
    ));
}
