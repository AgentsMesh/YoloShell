use crate::constants::*;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalProfile {
    pub schema_version: u16,
    pub state: PathBuf,
    pub host_home: PathBuf,
}

impl LocalProfile {
    pub fn from_application_support(base: &Path) -> Result<Self> {
        validate_path(base)?;
        let root = base.join(APPLICATION_DIRECTORY);
        let profile = Self {
            schema_version: SCHEMA_VERSION,
            state: root.join(STATE_DIRECTORY),
            host_home: root.join(HOST_DIRECTORY),
        };
        profile.validate()?;
        Ok(profile)
    }

    /// Reads an explicit private profile; never discovers or creates another profile.
    pub fn load(path: &Path) -> Result<Self> {
        validate_path(path)?;
        let profile: Self = yolo_paths::read_json(path)
            .with_context(|| format!("read local profile {}", path.display()))?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(self.schema_version == SCHEMA_VERSION, SCHEMA_INVALID);
        validate_path(&self.state)?;
        validate_path(&self.host_home)?;
        let state = yolo_paths::platform::platform_path(&self.state);
        let host_home = yolo_paths::platform::platform_path(&self.host_home);
        ensure!(
            !state.starts_with(&host_home) && !host_home.starts_with(&state),
            PROFILE_INVALID
        );
        Ok(())
    }
}

pub(crate) fn validate_path(path: &Path) -> Result<()> {
    yolo_paths::validation::validate_absolute(path)?;
    ensure!(
        path.to_str()
            .is_some_and(|text| !text.chars().any(char::is_control)),
        PROFILE_INVALID
    );
    Ok(())
}
