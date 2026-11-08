use super::super::constants::{SOCKET_ROOT_PERMISSION_MASK, SOCKET_ROOT_REQUIRED_MODE};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use yolo_runtime::paths::SOCKET_ROOT_ENVIRONMENT;

pub(super) fn inherited() -> PathBuf {
    let configured = std::env::var_os(SOCKET_ROOT_ENVIRONMENT)
        .map(PathBuf::from)
        .expect("runtime resilience test requires an isolated socket root");
    assert!(
        configured.is_absolute(),
        "isolated socket root is not absolute"
    );
    let root = configured.canonicalize().unwrap();
    let metadata = root.metadata().unwrap();
    assert!(metadata.is_dir(), "isolated socket root is not a directory");
    assert_eq!(
        metadata.permissions().mode() & SOCKET_ROOT_PERMISSION_MASK,
        SOCKET_ROOT_REQUIRED_MODE,
        "isolated socket root is not private"
    );
    root
}
