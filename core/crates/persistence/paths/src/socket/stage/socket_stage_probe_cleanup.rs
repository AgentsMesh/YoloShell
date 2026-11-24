use super::cleanup_io::{Inspection, SocketState, inspect, rename_exclusive, unlink};
use super::probe_target as target_proof;
use super::*;
use crate::paths::constants::{SOCKET_ENDPOINT_LINK_COUNT, SOCKET_PROBE_LINK_COUNT};
use crate::paths::directory::PrivateDirectory;
use anyhow::Result;
use std::ffi::CStr;

#[path = "socket_stage_probe_proof.rs"]
mod proof;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../test_support/integration/socket_stage_probe_cleanup_hooks.rs"]
mod test_hooks;
pub(super) use proof::{alias_proven, proven};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Recovery {
    Missing,
    Removed,
    Unproven,
}

pub(super) struct Hooks<'a> {
    pub(super) before_quarantine: &'a mut dyn FnMut() -> Result<()>,
    pub(super) after_quarantine: &'a mut dyn FnMut() -> Result<()>,
    pub(super) before_unlink: &'a mut dyn FnMut() -> Result<()>,
    pub(super) after_unlink: &'a mut dyn FnMut() -> Result<()>,
}

pub(super) fn cleanup_quarantine(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    hooks: &mut Hooks<'_>,
) -> Result<Recovery> {
    let (name, _) = target.probe_quarantine()?;
    let state = match inspect(directory, &name)? {
        Inspection::Missing => return Ok(Recovery::Missing),
        Inspection::Socket(state) if state.links == SOCKET_ENDPOINT_LINK_COUNT => {
            return remove_single(directory, target, &name, state, hooks);
        }
        Inspection::Socket(state) if state.links == SOCKET_PROBE_LINK_COUNT => state,
        Inspection::Socket(_) | Inspection::Unsafe => return Ok(Recovery::Unproven),
    };
    remove_proven(directory, target, endpoint, &name, state, hooks)
}

pub(super) fn cleanup_probe(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    expected: SocketState,
    hooks: &mut Hooks<'_>,
) -> Result<Recovery> {
    let (source, _) = target.stage(Kind::Probe)?;
    let (quarantine, _) = target.probe_quarantine()?;
    if !alias_proven(directory, target, endpoint, &source, expected)? {
        return Ok(Recovery::Unproven);
    }
    (hooks.before_quarantine)()?;
    if !alias_proven(directory, target, endpoint, &source, expected)?
        || !matches!(inspect(directory, &quarantine)?, Inspection::Missing)
    {
        return Ok(Recovery::Unproven);
    }
    #[cfg(all(test, feature = "yolo-integration-tests"))]
    test_hooks::run_before_rename()?;
    if !rename_exclusive(directory, &source, &quarantine)? {
        return Ok(Recovery::Unproven);
    }
    (hooks.after_quarantine)()?;
    directory.sync()?;
    if !alias_proven(directory, target, endpoint, &quarantine, expected)? {
        restore(directory, &source, &quarantine)?;
        return Ok(Recovery::Unproven);
    }
    if expected.links == SOCKET_ENDPOINT_LINK_COUNT {
        return remove_single(directory, target, &quarantine, expected, hooks);
    }
    remove_proven(directory, target, endpoint, &quarantine, expected, hooks)
}

pub(super) fn remove_proven(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    name: &CStr,
    expected: SocketState,
    hooks: &mut Hooks<'_>,
) -> Result<Recovery> {
    if !quarantine_proven(directory, target, endpoint, name, expected)? {
        return Ok(Recovery::Unproven);
    }
    (hooks.before_unlink)()?;
    if !quarantine_proven(directory, target, endpoint, name, expected)? {
        return Ok(Recovery::Unproven);
    }
    unlink(directory, name)?;
    (hooks.after_unlink)()?;
    directory.sync()?;
    if !matches!(inspect(directory, name)?, Inspection::Missing)
        || !target_proof::probe_slot_missing(directory, target)?
        || !target_proof::has_links(
            directory,
            target,
            endpoint,
            expected.identity,
            SOCKET_ENDPOINT_LINK_COUNT,
        )?
    {
        return Ok(Recovery::Unproven);
    }
    Ok(Recovery::Removed)
}

fn remove_single(
    directory: &PrivateDirectory,
    target: &Target,
    name: &CStr,
    expected: SocketState,
    hooks: &mut Hooks<'_>,
) -> Result<Recovery> {
    if !single_quarantine_proven(directory, target, name, expected)? {
        return Ok(Recovery::Unproven);
    }
    (hooks.before_unlink)()?;
    if !single_quarantine_proven(directory, target, name, expected)? {
        return Ok(Recovery::Unproven);
    }
    unlink(directory, name)?;
    (hooks.after_unlink)()?;
    directory.sync()?;
    if matches!(inspect(directory, name)?, Inspection::Missing)
        && target_proof::probe_slot_missing(directory, target)?
    {
        Ok(Recovery::Removed)
    } else {
        Ok(Recovery::Unproven)
    }
}

fn quarantine_proven(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    name: &CStr,
    expected: SocketState,
) -> Result<bool> {
    Ok(proven(directory, target, endpoint, name, expected)?
        && target_proof::probe_slot_missing(directory, target)?)
}

fn single_quarantine_proven(
    directory: &PrivateDirectory,
    target: &Target,
    name: &CStr,
    expected: SocketState,
) -> Result<bool> {
    Ok(
        matches!(inspect(directory, name)?, Inspection::Socket(actual) if actual == expected)
            && target_proof::probe_slot_missing(directory, target)?,
    )
}

fn restore(directory: &PrivateDirectory, source: &CStr, quarantine: &CStr) -> Result<()> {
    if rename_exclusive(directory, quarantine, source)? {
        directory.sync()?;
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
pub(crate) use test_hooks::install_before_rename;
