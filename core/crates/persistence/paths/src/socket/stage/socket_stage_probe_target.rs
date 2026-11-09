use super::cleanup_io::{Identity, Inspection, SocketState, inspect};
use super::{Kind, Target};
use crate::paths::constants::MAX_PRIVATE_DIRECTORY_ENTRIES;
use crate::paths::directory::{PrivateDirectory, entry_c_string};
use crate::paths::directory_stream::DirectoryStream;
use anyhow::{Context, Result};
use std::ffi::{CStr, CString};

const PROBE_PROOF_ERROR: &str = "socket probe alias target could not be proven";

pub(super) fn has_links(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
    expected: Identity,
    links: u64,
) -> Result<bool> {
    Ok(find(directory, target, endpoint)?
        .is_some_and(|(_, state)| state.identity == expected && state.links == links))
}

pub(super) fn probe_slot_missing(directory: &PrivateDirectory, target: &Target) -> Result<bool> {
    let (source, _) = target.stage(Kind::Probe)?;
    Ok(matches!(inspect(directory, &source)?, Inspection::Missing))
}

fn find(
    directory: &PrivateDirectory,
    target: &Target,
    endpoint: Option<&CStr>,
) -> Result<Option<(CString, SocketState)>> {
    if let Some(name) = endpoint {
        if !target.matches_endpoint(name) {
            return Ok(None);
        }
        let Inspection::Socket(state) = inspect(directory, name)? else {
            return Ok(None);
        };
        return Ok(Some((name.to_owned(), state)));
    }
    let mut stream = DirectoryStream::open(directory.descriptor())?;
    let mut entries = 0_usize;
    let mut matched = None;
    while let Some(name) = stream.next_name()? {
        entries = entries.checked_add(1).context(PROBE_PROOF_ERROR)?;
        if entries > MAX_PRIVATE_DIRECTORY_ENTRIES {
            return Ok(None);
        }
        let name = entry_c_string(&name)?;
        if !crate::paths::socket_path::is_endpoint_name(&name) || !target.matches_endpoint(&name) {
            continue;
        }
        let Inspection::Socket(state) = inspect(directory, &name)? else {
            return Ok(None);
        };
        if matched.replace((name, state)).is_some() {
            return Ok(None);
        }
    }
    Ok(matched)
}
