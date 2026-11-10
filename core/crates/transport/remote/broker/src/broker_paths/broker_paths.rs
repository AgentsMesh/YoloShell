use super::constants::*;
use super::security;
use crate::coordination::{CoordinationScope, lock_path};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::fmt::{Display, Formatter};
use std::mem::MaybeUninit;
use std::os::fd::RawFd;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

const ANCHOR_SEPARATOR: char = ':';
#[path = "broker_paths_existing.rs"]
mod existing;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrokerStateAnchor {
    device: u64,
    inode: u64,
}

impl BrokerStateAnchor {
    fn capture(directory: &PrivateDirectory) -> Result<Self> {
        Self::capture_descriptor(directory.descriptor())
    }

    fn capture_descriptor(descriptor: RawFd) -> Result<Self> {
        let mut metadata = MaybeUninit::<libc::stat>::uninit();
        let result = unsafe { libc::fstat(descriptor, metadata.as_mut_ptr()) };
        if result != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let metadata = unsafe { metadata.assume_init() };
        Ok(Self {
            device: metadata
                .st_dev
                .try_into()
                .context(BROKER_STATE_DEVICE_INVALID)?,
            inode: metadata.st_ino,
        })
    }
}

impl Display for BrokerStateAnchor {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}{ANCHOR_SEPARATOR}{}", self.device, self.inode)
    }
}

impl FromStr for BrokerStateAnchor {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let (device, inode) = value
            .split_once(ANCHOR_SEPARATOR)
            .ok_or_else(|| anyhow::anyhow!(BROKER_STATE_ANCHOR_INVALID))?;
        Ok(Self {
            device: device.parse()?,
            inode: inode.parse()?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct BrokerPaths {
    state_dir: PathBuf,
    instance_lock: PathBuf,
    launch_publication_lock: PathBuf,
    reconcile_lock: PathBuf,
    state: Arc<PrivateDirectory>,
    anchor: BrokerStateAnchor,
}

impl BrokerPaths {
    pub fn new(state_dir: impl Into<PathBuf>) -> Result<Self> {
        let state_dir = state_dir.into();
        let state = security::prepare_private_directory(&state_dir)?;
        Self::finish(state_dir, state)
    }

    pub fn new_expected(
        state_dir: impl Into<PathBuf>,
        expected: BrokerStateAnchor,
    ) -> Result<Self> {
        let state_dir = state_dir.into();
        security::validate_absolute(&state_dir, BROKER_STATE_DIRECTORY_LABEL)?;
        let state = PrivateDirectory::open(&state_dir)?;
        anyhow::ensure!(
            BrokerStateAnchor::capture(&state)? == expected,
            BROKER_STATE_ANCHOR_MISMATCH
        );
        Self::finish(state_dir, state)
    }

    fn finish(state_dir: PathBuf, state: PrivateDirectory) -> Result<Self> {
        let anchor = BrokerStateAnchor::capture(&state)?;
        let instance_lock = lock_path(CoordinationScope::BrokerInstance, &state_dir, None)?;
        let launch_publication_lock =
            lock_path(CoordinationScope::BrokerLaunchPublication, &state_dir, None)?;
        let reconcile_lock = lock_path(CoordinationScope::BrokerReconcile, &state_dir, None)?;
        let paths = Self {
            state_dir,
            instance_lock,
            launch_publication_lock,
            reconcile_lock,
            state: Arc::new(state),
            anchor,
        };
        security::validate_socket_path(&paths.runtime_socket())?;
        security::validate_socket_path(&paths.management_socket())?;
        Ok(paths)
    }

    #[cfg(test)]
    pub(super) fn new_with_coordination(
        state_dir: impl Into<PathBuf>,
        coordination: &Path,
    ) -> Result<Self> {
        let mut paths = Self::new(state_dir)?;
        PrivateDirectory::ensure(coordination)?;
        let instance = paths
            .instance_lock
            .file_name()
            .context(BROKER_COORDINATION_LOCK_NAME_ERROR)?
            .to_owned();
        let reconcile = paths
            .reconcile_lock
            .file_name()
            .context(BROKER_COORDINATION_LOCK_NAME_ERROR)?
            .to_owned();
        let publication = paths
            .launch_publication_lock
            .file_name()
            .context(BROKER_COORDINATION_LOCK_NAME_ERROR)?
            .to_owned();
        paths.instance_lock = coordination.join(instance);
        paths.launch_publication_lock = coordination.join(publication);
        paths.reconcile_lock = coordination.join(reconcile);
        Ok(paths)
    }

    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    pub fn state_anchor(&self) -> BrokerStateAnchor {
        self.anchor
    }

    pub fn verify_state(&self) -> Result<()> {
        self.state.verify_path()
    }

    pub(super) fn state_directory(&self) -> Result<PrivateDirectory> {
        self.state.try_clone()
    }

    pub fn runtime_socket(&self) -> PathBuf {
        self.state_dir.join(RUNTIME_SOCKET_FILE)
    }

    pub fn management_socket(&self) -> PathBuf {
        self.state_dir.join(MANAGEMENT_SOCKET_FILE)
    }

    pub(super) fn instance_lock(&self) -> &Path {
        &self.instance_lock
    }

    pub(super) fn launch_publication_lock(&self) -> &Path {
        &self.launch_publication_lock
    }

    pub(super) fn reconcile_lock(&self) -> &Path {
        &self.reconcile_lock
    }

    pub(super) fn generations(&self) -> PathBuf {
        self.state_dir.join(CONFIG_GENERATIONS_DIRECTORY)
    }

    pub(super) fn launch_authority(&self) -> PathBuf {
        self.state_dir
            .join(super::launch_authority::BROKER_LAUNCH_AUTHORITY_FILE)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/broker/broker_paths_failure_tests.rs"]
mod failure_tests;
