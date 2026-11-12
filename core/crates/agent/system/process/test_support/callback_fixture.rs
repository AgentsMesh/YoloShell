use super::super::fallback_observation::{
    CwdIdentity, ExecutableIdentity, ProcessEnvironmentObservation, ProcessIdentity,
    ProcessIdentityApi, ProcessObservationApi,
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};
#[path = "callback/constants.rs"]
mod constants;
pub(super) use constants::*;

pub(super) struct Api {
    pub identities: RefCell<HashMap<u32, ProcessIdentity>>,
    mutation: RefCell<Option<(u32, fn(&mut ProcessIdentity))>>,
    pub unavailable: Cell<bool>,
    pub change_peer_after_provider: Cell<bool>,
    peer_changed: Cell<bool>,
    pub identity_reads: Cell<usize>,
}

impl Api {
    pub fn new() -> Self {
        let identities = [
            identity(ROOT, UNRELATED, ROOT),
            identity(PROVIDER, ROOT, PROVIDER),
            identity(LAUNCHER, PROVIDER, LAUNCHER),
            identity(PEER, LAUNCHER, LAUNCHER),
        ]
        .into_iter()
        .map(|identity| (identity.pid, identity))
        .collect();
        Self {
            identities: RefCell::new(identities),
            mutation: RefCell::new(None),
            unavailable: Cell::new(false),
            change_peer_after_provider: Cell::new(false),
            peer_changed: Cell::new(false),
            identity_reads: Cell::new(0),
        }
    }
    pub fn mutate_during_observation(&self, pid: u32, change: fn(&mut ProcessIdentity)) {
        self.mutation.replace(Some((pid, change)));
    }
}

impl ProcessIdentityApi for Api {
    fn identity(&self, pid: u32) -> Option<ProcessIdentity> {
        self.identity_reads.set(self.identity_reads.get() + 1);
        self.identities.borrow().get(&pid).cloned()
    }
}

impl ProcessObservationApi for Api {
    fn argv(&self, pid: u32) -> anyhow::Result<Vec<String>> {
        if let Some((target, change)) = self.mutation.borrow_mut().take() {
            change(self.identities.borrow_mut().get_mut(&target).unwrap());
        }
        if pid == PROVIDER && self.change_peer_after_provider.get() {
            self.peer_changed.set(true);
        }
        Ok(vec![EXECUTABLE.into()])
    }
    fn environment(
        &self,
        pid: u32,
        keys: &[&str],
    ) -> anyhow::Result<ProcessEnvironmentObservation> {
        assert_eq!(keys, [ENVIRONMENT_KEY]);
        if self.unavailable.get() {
            return Ok(ProcessEnvironmentObservation::Unavailable);
        }
        let value = if pid == PEER && self.peer_changed.get() {
            CHANGED_ENVIRONMENT_VALUE
        } else {
            ENVIRONMENT_VALUE
        };
        Ok(ProcessEnvironmentObservation::Selected(BTreeMap::from([(
            ENVIRONMENT_KEY.into(),
            value.into(),
        )])))
    }
    fn evidence_paths(&self, _pid: u32) -> Option<Vec<PathBuf>> {
        Some(Vec::new())
    }
}

pub(super) fn identity(pid: u32, parent_pid: u32, group: u32) -> ProcessIdentity {
    ProcessIdentity {
        pid,
        parent_pid,
        process_device: DEVICE,
        process_inode: u64::from(pid),
        start_time: START_TIME,
        process_group: i32::try_from(group).unwrap(),
        foreground_group: i32::try_from(PROVIDER).unwrap(),
        executable: EXECUTABLE.into(),
        executable_file: ExecutableIdentity {
            path: EXECUTABLE_PATH.into(),
            device: DEVICE,
            inode: u64::from(pid),
            file_type: libc::S_IFREG as u32,
        },
        cwd: CwdIdentity {
            path: CWD.into(),
            device: DEVICE,
            inode: u64::from(pid),
        },
    }
}
