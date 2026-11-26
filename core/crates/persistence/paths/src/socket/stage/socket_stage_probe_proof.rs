use super::target_proof;
use super::{Inspection, SocketState, Target, inspect};
use crate::paths::constants::{SOCKET_ENDPOINT_LINK_COUNT, SOCKET_PROBE_LINK_COUNT};
use crate::paths::directory::PrivateDirectory;
use anyhow::Result;
use std::ffi::CStr;

pub(in super::super) fn proven(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    alias: &CStr,
    expected: SocketState,
) -> Result<bool> {
    if expected.links != SOCKET_PROBE_LINK_COUNT
        || !matches!(inspect(directory, alias)?, Inspection::Socket(actual) if actual == expected)
    {
        return Ok(false);
    }
    target_proof::has_links(
        directory,
        target,
        endpoint,
        expected.identity,
        SOCKET_PROBE_LINK_COUNT,
    )
}

pub(in super::super) fn alias_proven(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    alias: &CStr,
    expected: SocketState,
) -> Result<bool> {
    if expected.links == SOCKET_ENDPOINT_LINK_COUNT {
        return Ok(matches!(
            inspect(directory, alias)?,
            Inspection::Socket(actual) if actual == expected
        ));
    }
    proven(directory, target, endpoint, alias, expected)
}
