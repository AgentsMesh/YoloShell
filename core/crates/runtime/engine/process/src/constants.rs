pub use yolo_process_identity::constants::UNKNOWN_BOOT_ID_PREFIX;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) const PROCESS_INVENTORY_PROGRAM: &str = "ps";
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub(super) const PROCESS_INVENTORY_ARGS: &[&str] = &["-axo", "pid=,ppid=,pgid="];
pub(super) const MAX_PROCESS_INVENTORY_IDS: usize = 32_768;
pub(super) const EVENT_RECONCILIATION_LIMIT: usize = 4;
pub(super) const POST_KILL_EXACT_GENERATION_POLL_LIMIT: usize = 4;
pub(super) const PROCESS_INVENTORY_CAPTURE_ATTEMPTS: usize = 4;
pub(super) const PROCESS_INVENTORY_RETRY_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(10);
pub(super) const PTY_MASTER_ALIAS_PROOF_MISMATCH_DETAIL: &str =
    "PTY master-close proof belongs to another process closure";
pub(super) const NO_PARENT_LINEAGE_ID: u64 = 0;
pub const PTY_SESSION_CAPABILITY_ENV: &str = "YOLO_PTY_SESSION_CAPABILITY";
pub const PROCESS_DISCOVERY_CAPABILITY_ENV: &str = "YOLO_PROCESS_DISCOVERY_CAPABILITY";
#[cfg(all(test, target_os = "linux"))]
pub(super) const TEST_PROCESS_DISCOVERY_CAPABILITY_VALUE: &str = "exact";
#[cfg(target_os = "macos")]
pub(super) const PROCESS_DISCOVERY_MANAGER_CAPABILITY_ENV: &str =
    "YOLO_PROCESS_DISCOVERY_MANAGER_CAPABILITY";
#[cfg(target_os = "macos")]
pub const PROCESS_DESCRIPTOR_CAPABILITY_FD_ENV: &str = "YOLO_PROCESS_DESCRIPTOR_CAPABILITY_FD";
#[cfg(target_os = "macos")]
pub(super) const PID_LIST_SENTINEL_COUNT: usize = 1;
#[cfg(target_os = "macos")]
pub(super) const MAX_PTY_PROOF_FILE_DESCRIPTORS: usize = 4_096;
#[cfg(target_os = "macos")]
pub(super) const MAX_PTY_PROOF_CANDIDATES: usize =
    crate::runtime_constants::MAX_NAMESPACE_LOSS_PTY_HOLDERS;
#[cfg(target_os = "macos")]
pub(super) const MAX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS: usize = 32_768;
#[cfg(target_os = "macos")]
pub(super) const MAX_PTY_PROOF_ENVIRONMENT_BYTES: usize = 8 * 1_024 * 1_024;
#[cfg(target_os = "macos")]
pub(super) const MAX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES: usize = 64 * 1_024 * 1_024;
#[cfg(target_os = "macos")]
pub(super) const MACOS_PROCESS_ARGUMENT_FIELD_SEPARATOR_BYTES: usize = 1;
#[cfg(target_os = "macos")]
pub(super) const MACOS_PROCESS_ARGUMENT_POPULATED_EXTENT_BYTES: usize = 2;
#[cfg(target_os = "macos")]
pub(super) const MACOS_SYSCTL_SUCCESS: libc::c_int = 0;
#[cfg(target_os = "macos")]
pub(super) const MACOS_VNODE_PATH_INFO_FLAVOR: i32 = 2;
#[cfg(target_os = "macos")]
pub(super) const MACOS_PIPE_INFO_FLAVOR: i32 = 6;
#[cfg(target_os = "macos")]
// Darwin <spawn.h>: POSIX_SPAWN_SETSID is not exported by the current libc crate.
pub(super) const MACOS_POSIX_SPAWN_SETSID_FLAG: libc::c_short = 0x0400;
#[cfg(target_os = "macos")]
pub(super) use yolo_process_identity::constants::MACOS_PROC_PIDINFO_NO_ARGUMENT;
#[cfg(target_os = "macos")]
pub(super) const MACOS_PROC_ALL_PIDS: u32 = 1;
#[cfg(target_os = "macos")]
pub(super) const MACOS_PROC_PGRP_ONLY: u32 = 2;
#[cfg(target_os = "macos")]
pub(super) const MACOS_PROC_UID_ONLY: u32 = 4;
#[cfg(target_os = "linux")]
pub(super) use yolo_process_identity::constants::LINUX_START_TIME_FIELD_OFFSET;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PROC_INVENTORY_ENTRY_LIMIT: usize = 65_536;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PROC_STAT_BYTES: usize = 4_096;
#[cfg(target_os = "linux")]
pub(super) use yolo_process_identity::constants::{
    LINUX_PROC_ENVIRONMENT_ENTRY, LINUX_PROC_STATUS_ENTRY,
};
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_CANDIDATES_PER_BATCH: usize = 64;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_RETAINED_AUTHORITIES: usize = 256;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_RETAINED_FD_RESERVE: usize = 256;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_TRANSIENT_FD_BUDGET: usize = 2;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_OPEN_FD_SCAN_LIMIT: usize = 65_536;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_STATUS_BYTES: usize = 256 * 1_024;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_ENVIRONMENT_BYTES: usize = 8 * 1_024 * 1_024;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_TOTAL_ENVIRONMENT_BYTES: usize = 64 * 1_024 * 1_024;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_FILE_DESCRIPTORS: usize = 4_096;
#[cfg(target_os = "linux")]
pub(super) const LINUX_PTY_PROOF_TOTAL_FILE_DESCRIPTORS: usize = 32_768;
