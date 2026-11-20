use std::time::Duration;

pub(super) const DELEGATED_ROOT_ENVIRONMENT: &str = "YOLO_CGROUP_DELEGATED_ROOT";
pub(super) const CGROUP2_SUPER_MAGIC: libc::c_long = 0x6367_7270;
pub(super) const CGROUP_CONTROLLERS_FILE: &str = "cgroup.controllers";
pub(super) const CGROUP_EVENTS_FILE: &str = "cgroup.events";
pub(super) const CGROUP_FREEZE_FILE: &str = "cgroup.freeze";
pub(super) const CGROUP_KILL_FILE: &str = "cgroup.kill";
pub(super) const CGROUP_PROCESSES_FILE: &str = "cgroup.procs";
pub(super) const CGROUP_SUBTREE_CONTROL_FILE: &str = "cgroup.subtree_control";
pub(super) const PIDS_MAX_FILE: &str = "pids.max";
pub(super) const PIDS_CONTROLLER: &str = "pids";
pub(super) const PIDS_ENABLE_COMMAND: &str = "+pids\n";
pub(super) const PIDS_MAX_VALUE: &str = "256\n";
pub(super) const LEAF_PREFIX: &str = "yolo-session-";
pub(super) const RECORD_FILE_PREFIX: &str = "linux-containment-";
pub(super) const RECORD_FILE_SUFFIX: &str = ".json";
pub(super) const RECORD_SCHEMA_VERSION: u16 = 3;
pub(super) const MAX_CONTROL_BYTES: usize = 64 * 1_024;
pub(super) const MAX_FDINFO_BYTES: usize = 16 * 1_024;
pub(super) const FDINFO_MOUNT_ID_PREFIX: &str = "mnt_id:";
pub(super) const REQUIRED_EVENT_POPULATED: &str = "populated";
pub(super) const REQUIRED_EVENT_FROZEN: &str = "frozen";
pub(super) const FREEZE_VALUE: &str = "1\n";
pub(super) const KILL_VALUE: &str = "1\n";
pub(super) const FREEZE_POLL_INTERVAL: Duration = Duration::from_millis(10);
pub(super) const FREEZE_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const OPEN_MODE_UNUSED: libc::mode_t = 0;
pub(super) const CREATE_DIRECTORY_MODE: libc::mode_t = 0o700;
pub(super) const INVALID_DESCRIPTOR: libc::c_int = -1;

pub(crate) fn record_path(
    session_directory: &std::path::Path,
    runtime_epoch: uuid::Uuid,
) -> std::path::PathBuf {
    session_directory.join(format!(
        "{RECORD_FILE_PREFIX}{runtime_epoch}{RECORD_FILE_SUFFIX}"
    ))
}

pub(super) fn session_leaf_name(terminal_id: uuid::Uuid, runtime_epoch: uuid::Uuid) -> String {
    format!("{LEAF_PREFIX}{terminal_id}-{runtime_epoch}")
}
