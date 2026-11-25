use super::cleanup_io::{Inspection, SocketState, inspect, unlink};
use super::probe_cleanup::{Hooks, Recovery};
use super::*;
use crate::paths::constants::{SOCKET_ENDPOINT_LINK_COUNT, SOCKET_PROBE_LINK_COUNT};
use crate::paths::directory::PrivateDirectory;
use anyhow::Result;
use std::ffi::CStr;

const STAGE_CHANGED: &str = "managed socket publication stage changed";

pub(crate) fn cleanup_known(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: &CStr,
) -> Result<bool> {
    let mut before_quarantine = || Ok(());
    let mut after_quarantine = || Ok(());
    let mut before_unlink = || Ok(());
    let mut after_unlink = || Ok(());
    let mut hooks = Hooks {
        before_quarantine: &mut before_quarantine,
        after_quarantine: &mut after_quarantine,
        before_unlink: &mut before_unlink,
        after_unlink: &mut after_unlink,
    };
    cleanup_with_hooks(directory, target, Some(endpoint), &mut hooks)
}

pub(super) fn cleanup_with_hooks(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    hooks: &mut Hooks<'_>,
) -> Result<bool> {
    if probe_cleanup::cleanup_quarantine(directory, target, endpoint, hooks)? == Recovery::Unproven
    {
        return Ok(false);
    }
    let mut bind_stages = Vec::new();
    for kind in [Kind::Bind, Kind::Probe] {
        let (name, _) = target.stage(kind)?;
        match inspect(directory, &name)? {
            Inspection::Missing => {}
            Inspection::Socket(state)
                if kind == Kind::Bind && state.links == SOCKET_ENDPOINT_LINK_COUNT =>
            {
                bind_stages.push((name, state));
            }
            Inspection::Socket(state)
                if kind == Kind::Probe
                    && matches!(
                        state.links,
                        SOCKET_ENDPOINT_LINK_COUNT | SOCKET_PROBE_LINK_COUNT
                    ) =>
            {
                if probe_cleanup::cleanup_probe(directory, target, endpoint, state, hooks)?
                    != Recovery::Removed
                {
                    return Ok(false);
                }
            }
            Inspection::Socket(_) | Inspection::Unsafe => return Ok(false),
        }
    }
    remove_bind_stages(directory, &bind_stages)?;
    residue_is_clear(directory, target)
}

fn remove_bind_stages(
    directory: &PrivateDirectory,
    stages: &[(std::ffi::CString, SocketState)],
) -> Result<()> {
    for (name, state) in stages {
        anyhow::ensure!(
            matches!(inspect(directory, name)?, Inspection::Socket(actual) if actual == *state),
            STAGE_CHANGED
        );
        unlink(directory, name)?;
    }
    if !stages.is_empty() {
        directory.sync()?;
    }
    Ok(())
}

fn residue_is_clear(directory: &PrivateDirectory, target: &Target) -> Result<bool> {
    for name in target.residue_names()? {
        if !matches!(inspect(directory, &name)?, Inspection::Missing) {
            return Ok(false);
        }
    }
    Ok(true)
}
