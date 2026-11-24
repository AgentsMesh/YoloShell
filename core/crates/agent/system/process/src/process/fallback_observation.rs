use super::super::ObservedProcess;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CwdIdentity {
    pub path: PathBuf,
    pub device: u64,
    pub inode: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutableIdentity {
    pub path: PathBuf,
    pub device: u64,
    pub inode: u64,
    pub(super) file_type: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ProcessIdentity {
    pub pid: u32,
    pub(super) process_device: u64,
    pub(super) process_inode: u64,
    pub(super) start_time: u64,
    pub parent_pid: u32,
    pub process_group: i32,
    pub foreground_group: i32,
    pub executable: String,
    pub(super) executable_file: ExecutableIdentity,
    pub(super) cwd: CwdIdentity,
}

impl ProcessIdentity {
    fn is_valid_for(&self, process: &ProcessRow) -> bool {
        self.pid == process.pid
            && self.parent_pid == process.parent_pid
            && self.process_group == process.process_group
            && self.foreground_group == process.foreground_group
            && self.executable == process.executable
            && self.executable_file.path.is_absolute()
            && self.executable_file.file_type == libc::S_IFREG as u32
            && process.pid >= super::super::constants::MIN_PROCESS_PID
            && self.start_time != u64::default()
            && self.cwd.path.is_absolute()
            && process
                .inventory_identity
                .as_ref()
                .is_none_or(|expected| expected == self)
    }
}

pub(super) struct ProcessRow {
    pub pid: u32,
    pub parent_pid: u32,
    pub process_group: i32,
    pub foreground_group: i32,
    pub executable: String,
    pub(super) inventory_identity: Option<ProcessIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProcessEnvironmentObservation {
    Selected(BTreeMap<String, String>),
    Unavailable,
}

pub(super) trait ProcessIdentityApi {
    fn identity(&self, pid: u32) -> Option<ProcessIdentity>;
}

pub(super) trait ProcessObservationApi: ProcessIdentityApi {
    fn argv(&self, pid: u32) -> Result<Vec<String>>;
    fn environment(&self, pid: u32, keys: &[&str]) -> Result<ProcessEnvironmentObservation>;
    fn evidence_paths(&self, pid: u32) -> Option<Vec<PathBuf>>;
}

pub(super) fn observe_process(
    api: &impl ProcessObservationApi,
    process: ProcessRow,
    environment_keys: &[&str],
) -> Option<ObservedProcess> {
    let before = api.identity(process.pid)?;
    if !before.is_valid_for(&process) {
        return None;
    }
    let argv = api.argv(process.pid).ok()?;
    let environment = api.environment(process.pid, environment_keys).ok()?;
    let evidence_paths = api.evidence_paths(process.pid);
    let stable_argv = api.argv(process.pid).ok()?;
    let stable_environment = api.environment(process.pid, environment_keys).ok()?;
    let stable_evidence_paths = api.evidence_paths(process.pid);
    let after = api.identity(process.pid)?;
    if before != after
        || argv != stable_argv
        || environment != stable_environment
        || evidence_paths != stable_evidence_paths
    {
        return None;
    }
    let ProcessEnvironmentObservation::Selected(environment) = environment else {
        return None;
    };
    Some(ObservedProcess {
        pid: process.pid,
        start_time: before.start_time,
        parent_pid: process.parent_pid,
        process_group: process.process_group,
        foreground_group: process.foreground_group,
        executable: process.executable,
        argv,
        environment: Some(environment),
        cwd: Some(before.cwd.path),
        evidence_paths,
    })
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_observation_identity_tests.rs"]
mod identity_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_observation_reuse_tests.rs"]
mod reuse_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_observation_snapshot_tests.rs"]
mod snapshot_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../integration_tests/process/fallback_observation_test_support.rs"]
pub(super) mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/process/fallback_observation_tests.rs"]
mod tests;
