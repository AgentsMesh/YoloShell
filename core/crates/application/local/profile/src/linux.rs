use crate::{LocalProfile, constants::*, profile::validate_path};
use anyhow::{Context, Result};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
pub(super) fn current() -> Result<LocalProfile> {
    from_environment(
        std::env::var_os(XDG_STATE_HOME_ENVIRONMENT).as_deref(),
        std::env::var_os(HOME_ENVIRONMENT).as_deref(),
    )
}

/// Resolve without changing the environment or creating service directories.
/// A configured invalid state home is an error, never a different profile.
pub(crate) fn from_environment(
    state_home: Option<&OsStr>,
    home: Option<&OsStr>,
) -> Result<LocalProfile> {
    let base = match state_home.filter(|value| !value.is_empty()) {
        Some(value) => PathBuf::from(value),
        None => {
            let home = Path::new(
                home.filter(|value| !value.is_empty())
                    .context(LINUX_HOME_UNAVAILABLE)?,
            );
            validate_path(home)?;
            home.join(LINUX_STATE_HOME_DIRECTORY)
        }
    };
    validate_path(&base)?;
    let root = base.join(LINUX_APPLICATION_DIRECTORY);
    let profile = LocalProfile {
        schema_version: SCHEMA_VERSION,
        state: root.join(STATE_DIRECTORY),
        host_home: root.join(LINUX_HOST_DIRECTORY),
    };
    profile.validate()?;
    Ok(profile)
}
