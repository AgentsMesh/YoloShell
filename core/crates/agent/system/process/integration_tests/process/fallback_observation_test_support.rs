use super::{
    CwdIdentity, ExecutableIdentity, ProcessEnvironmentObservation, ProcessIdentity,
    ProcessIdentityApi, ProcessObservationApi, ProcessRow,
};
use anyhow::{Result, anyhow, ensure};
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;

pub(super) const TEST_CWD: &str = "/srv/project";
const TEST_ENVIRONMENT_FAILURE: &str = "injected environment read failure";
const TEST_ENVIRONMENT_KEYS_MISMATCH: &str = "unexpected process environment keys";
const TEST_ARGV_FAILURE: &str = "injected argv read failure";
pub(super) const TEST_EVIDENCE: &str = "/srv/agent/session.jsonl";
pub(super) const TEST_EXECUTABLE_PATH: &str = "/opt/agent/bin/codex";
pub(super) const TEST_HOME: &str = "/srv/codex";
pub(super) const TEST_OTHER_CWD: &str = "/srv/replaced";
pub(super) const TEST_RELATIVE_CWD: &str = "relative-project";
pub(super) const TEST_DEVICE: u64 = 31;
pub(super) const TEST_FOREGROUND_GROUP: i32 = 43;
pub(super) const TEST_INODE: u64 = 47;
const TEST_PARENT_PID: u32 = 41;
pub(super) const TEST_PID: u32 = 42;
pub(super) const TEST_PROCESS_GROUP: i32 = TEST_FOREGROUND_GROUP;
pub(super) const TEST_PROCESS_DEVICE: u64 = 59;
pub(super) const TEST_PROCESS_INODE: u64 = 61;
pub(super) const TEST_START_TIME: u64 = 53;
pub(super) const IDENTITY_CHANGE: u64 = 1;

pub(super) struct ScriptedApi {
    identities: RefCell<VecDeque<Option<ProcessIdentity>>>,
    argv: RefCell<VecDeque<Option<Vec<String>>>>,
    environment: RefCell<VecDeque<EnvironmentOutcome>>,
    evidence_paths: RefCell<VecDeque<Option<Vec<PathBuf>>>>,
    expected_keys: Vec<&'static str>,
}

#[derive(Clone)]
pub(super) enum EnvironmentOutcome {
    Invalid,
    Selected(BTreeMap<String, String>),
    Unavailable,
}

impl ProcessIdentityApi for ScriptedApi {
    fn identity(&self, _pid: u32) -> Option<ProcessIdentity> {
        self.identities.borrow_mut().pop_front().flatten()
    }
}

impl ProcessObservationApi for ScriptedApi {
    fn argv(&self, _pid: u32) -> Result<Vec<String>> {
        self.argv
            .borrow_mut()
            .pop_front()
            .flatten()
            .ok_or_else(|| anyhow!(TEST_ARGV_FAILURE))
    }

    fn environment(&self, _pid: u32, keys: &[&str]) -> Result<ProcessEnvironmentObservation> {
        ensure!(keys == self.expected_keys, TEST_ENVIRONMENT_KEYS_MISMATCH);
        let outcome = self
            .environment
            .borrow_mut()
            .pop_front()
            .ok_or_else(|| anyhow!(TEST_ENVIRONMENT_FAILURE))?;
        match outcome {
            EnvironmentOutcome::Invalid => Err(anyhow!(TEST_ENVIRONMENT_FAILURE)),
            EnvironmentOutcome::Selected(values) => {
                Ok(ProcessEnvironmentObservation::Selected(values))
            }
            EnvironmentOutcome::Unavailable => Ok(ProcessEnvironmentObservation::Unavailable),
        }
    }

    fn evidence_paths(&self, _pid: u32) -> Option<Vec<PathBuf>> {
        self.evidence_paths.borrow_mut().pop_front().unwrap_or(None)
    }
}

pub(super) fn api(
    identities: impl IntoIterator<Item = Option<ProcessIdentity>>,
    environment: EnvironmentOutcome,
    expected_keys: Vec<&'static str>,
) -> ScriptedApi {
    let snapshots = stable_snapshots(environment);
    api_with_snapshots(identities, snapshots, expected_keys)
}

pub(super) struct ObservationSnapshots {
    pub(super) argv: [Option<Vec<String>>; 2],
    pub(super) environment: [EnvironmentOutcome; 2],
    pub(super) evidence_paths: [Option<Vec<PathBuf>>; 2],
}

pub(super) fn stable_snapshots(environment: EnvironmentOutcome) -> ObservationSnapshots {
    let argv = vec![
        crate::agent::codex::constants::EXECUTABLE.to_owned(),
        crate::agent::codex::constants::RESUME_ARGUMENT.to_owned(),
    ];
    let evidence = Some(vec![PathBuf::from(TEST_EVIDENCE)]);
    ObservationSnapshots {
        argv: [Some(argv.clone()), Some(argv)],
        environment: [environment.clone(), environment],
        evidence_paths: [evidence.clone(), evidence],
    }
}

pub(super) fn api_with_snapshots(
    identities: impl IntoIterator<Item = Option<ProcessIdentity>>,
    snapshots: ObservationSnapshots,
    expected_keys: Vec<&'static str>,
) -> ScriptedApi {
    ScriptedApi {
        identities: RefCell::new(identities.into_iter().collect()),
        argv: RefCell::new(snapshots.argv.into_iter().collect()),
        environment: RefCell::new(snapshots.environment.into_iter().collect()),
        evidence_paths: RefCell::new(snapshots.evidence_paths.into_iter().collect()),
        expected_keys,
    }
}

pub(super) fn stable_identities() -> [Option<ProcessIdentity>; 2] {
    [Some(identity()), Some(identity())]
}

pub(in crate::source) fn identity() -> ProcessIdentity {
    ProcessIdentity {
        pid: TEST_PID,
        process_device: TEST_PROCESS_DEVICE,
        process_inode: TEST_PROCESS_INODE,
        start_time: TEST_START_TIME,
        parent_pid: TEST_PARENT_PID,
        process_group: TEST_PROCESS_GROUP,
        foreground_group: TEST_FOREGROUND_GROUP,
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        executable_file: ExecutableIdentity {
            path: TEST_EXECUTABLE_PATH.into(),
            device: TEST_PROCESS_DEVICE,
            inode: TEST_PROCESS_INODE,
            file_type: libc::S_IFREG as u32,
        },
        cwd: CwdIdentity {
            path: TEST_CWD.into(),
            device: TEST_DEVICE,
            inode: TEST_INODE,
        },
    }
}

pub(super) fn row() -> ProcessRow {
    ProcessRow {
        pid: TEST_PID,
        parent_pid: TEST_PARENT_PID,
        process_group: TEST_PROCESS_GROUP,
        foreground_group: TEST_FOREGROUND_GROUP,
        executable: crate::agent::codex::constants::EXECUTABLE.to_owned(),
        inventory_identity: Some(identity()),
    }
}
