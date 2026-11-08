use super::*;
use crate::paths::constants::UNIX_SOCKET_PATH_MAX_BYTES;
use std::os::unix::ffi::OsStrExt;

const DIRECTORY_PREFIX: &str = "ysp-";
const LONG_COMPONENT_BYTE: u8 = b'x';
const PATH_SEPARATOR_BYTES: usize = 1;
const TARGET_NAME: &str = "s";

#[test]
fn a_parent_without_stage_room_is_rejected_before_bind() {
    let parent = Path::new(crate::paths::constants::SOCKET_ROOT_PREFIX)
        .parent()
        .unwrap();
    let root = tempfile::Builder::new()
        .prefix(DIRECTORY_PREFIX)
        .tempdir_in(parent)
        .unwrap();
    let target_name = CString::new(TARGET_NAME).unwrap();
    let stage_target = socket_stage::Target::new(root.path(), &target_name).unwrap();
    let (stage_name, _) = stage_target.stage(socket_stage::Kind::Bind).unwrap();
    let parent_budget = UNIX_SOCKET_PATH_MAX_BYTES
        .checked_sub(PATH_SEPARATOR_BYTES + stage_name.to_bytes().len())
        .unwrap();
    let component_length = parent_budget
        .checked_sub(root.path().as_os_str().as_bytes().len() + PATH_SEPARATOR_BYTES)
        .unwrap();
    let component = String::from_utf8(vec![LONG_COMPONENT_BYTE; component_length]).unwrap();
    let long_parent = root.path().join(component);
    let directory = PrivateDirectory::ensure(&long_parent).unwrap();
    let target = long_parent.join(TARGET_NAME);
    assert!(target.as_os_str().as_bytes().len() < UNIX_SOCKET_PATH_MAX_BYTES);

    let error = SocketTarget::new(directory, &target).err().unwrap();

    assert!(format!("{error:#}").contains(STAGE_PATH_BUDGET_ERROR));
    assert!(!target.exists());
}
