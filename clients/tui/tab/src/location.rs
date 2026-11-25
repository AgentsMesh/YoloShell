use crate::constants::*;
use anyhow::{Context, Result};
use std::path::PathBuf;
use yolo_hub_identity::HubId;

pub fn default_directory(hub: HubId) -> Result<PathBuf> {
    resolved(
        std::env::var_os(CLIENT_STATE_DIRECTORY_ENV),
        std::env::var_os(HOME_ENV),
        hub,
    )
}

pub(crate) fn resolved(
    override_path: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    hub: HubId,
) -> Result<PathBuf> {
    if let Some(path) = override_path {
        let path = PathBuf::from(path);
        yolo_paths::validation::validate_home(&path)?;
        return Ok(path);
    }
    from_home(home, hub)
}

pub(crate) fn from_home(base: Option<std::ffi::OsString>, hub: HubId) -> Result<PathBuf> {
    let base = base.map(PathBuf::from).context(HOME_UNAVAILABLE)?;
    location(base, hub)
}
pub(crate) fn location(mut base: PathBuf, hub: HubId) -> Result<PathBuf> {
    yolo_paths::validation::validate_absolute(&base)?;
    for component in APPLICATION_ROOT {
        base.push(component);
    }
    Ok(base.join(hub.to_string()))
}
