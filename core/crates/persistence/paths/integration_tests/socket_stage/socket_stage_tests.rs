use super::*;

const UPPERCASE_STAGE: &str = ".b-0000000000000000000000000000000A";
const LEGACY_BIND_STAGE: &str = ".yolo-socket-bind-00000000-0000-0000-0000-000000000000";

#[test]
fn production_stage_names_are_short_canonical_and_recognized() {
    let parent = Path::new(crate::paths::constants::SOCKET_ROOT_PREFIX);
    let name = CString::new(crate::paths::constants::HOST_SOCKET_FILE_NAME).unwrap();
    let target = Target::new(parent, &name).unwrap();
    for kind in [Kind::Bind, Kind::Probe] {
        let (name, path) = target.stage(kind).unwrap();
        assert!(is_managed_stage(&name));
        assert!(path.as_os_str().len() < crate::paths::constants::UNIX_SOCKET_PATH_MAX_BYTES);
    }
}

#[test]
fn noncanonical_and_legacy_stage_names_are_not_managed() {
    for value in [UPPERCASE_STAGE, LEGACY_BIND_STAGE] {
        let name = CString::new(value).unwrap();
        assert!(!is_managed_stage(&name));
    }
}

include!("socket_stage_probe_commit_tests.rs");
