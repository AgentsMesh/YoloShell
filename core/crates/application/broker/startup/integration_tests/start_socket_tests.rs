use super::super::socket;
use crate::fixture_constants as fixture;
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn existing_socket_inspection_propagates_non_absence_errors() {
    let root = tempfile::tempdir().unwrap();
    let blocked = root.path().join(fixture::BLOCKED_DIRECTORY);
    fs::create_dir(&blocked).unwrap();
    fs::set_permissions(
        &blocked,
        fs::Permissions::from_mode(fixture::NO_ACCESS_MODE),
    )
    .unwrap();
    let result = socket::validate_existing(&blocked.join(fixture::BLOCKED_ENDPOINT));
    fs::set_permissions(
        &blocked,
        fs::Permissions::from_mode(fixture::PRIVATE_DIRECTORY_MODE),
    )
    .unwrap();
    assert!(result.is_err());
}
