use anyhow::{Result, ensure};
use std::{
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};
use yolo_remote_broker::{BrokerLaunchAuthority, BrokerLaunchDeadline, BrokerStateAnchor};

const INVALID_PATH: &str = "broker child launch path is invalid";

/// Validated input description; opening state and acquiring authority remain separate operations.
#[derive(Clone, Debug)]
pub struct ChildLaunch {
    config: PathBuf,
    state_dir: PathBuf,
    config_revision: String,
    state_anchor: BrokerStateAnchor,
    authority: BrokerLaunchAuthority,
    startup_deadline: BrokerLaunchDeadline,
}

/// Describes where the caller will read credentials at its existing admission boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CredentialSource {
    None,
    PrivateStdin,
}

impl ChildLaunch {
    pub fn new(
        config: PathBuf,
        state_dir: PathBuf,
        config_revision: String,
        state_anchor: BrokerStateAnchor,
        authority: BrokerLaunchAuthority,
        startup_deadline: BrokerLaunchDeadline,
    ) -> Result<Self> {
        for path in [&config, &state_dir] {
            yolo_paths_core::validation::validate_absolute(path)?;
            ensure!(
                path.file_name().is_some() && !path.as_os_str().as_bytes().contains(&u8::default()),
                INVALID_PATH
            );
        }
        yolo_remote_broker::validate_broker_config_revision(&config_revision)?;
        Ok(Self {
            config,
            state_dir,
            config_revision,
            state_anchor,
            authority,
            startup_deadline,
        })
    }

    pub fn config(&self) -> &Path {
        &self.config
    }
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }
    pub fn config_revision(&self) -> &str {
        &self.config_revision
    }
    pub fn state_anchor(&self) -> BrokerStateAnchor {
        self.state_anchor
    }
    pub fn authority(&self) -> BrokerLaunchAuthority {
        self.authority
    }
    pub fn startup_deadline(&self) -> &BrokerLaunchDeadline {
        &self.startup_deadline
    }
}
