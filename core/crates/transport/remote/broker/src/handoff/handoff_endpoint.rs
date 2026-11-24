use super::constants::*;
use super::socket_directory::SocketDirectory;
use super::socket_identity::SocketIdentity;
use super::socket_probe::{BrokerSocketProbe, probe_broker_socket};
use anyhow::{Context, Result, bail};
use std::ffi::{CStr, CString};
use std::path::Path;

pub(super) async fn retire_endpoint(
    directory: &SocketDirectory,
    name: &CStr,
    path: &Path,
    expected: Option<SocketIdentity>,
) -> Result<bool> {
    let Some(expected) = expected else {
        return Ok(true);
    };
    if directory
        .identity(name)
        .context(BROKER_SOCKET_RETIRE_INSPECT_CONTEXT)?
        != Some(expected)
    {
        return Ok(true);
    }
    finish_retirement_probe(directory, name, expected, probe_broker_socket(path).await)
}

pub(super) fn finish_retirement_probe(
    directory: &SocketDirectory,
    name: &CStr,
    expected: SocketIdentity,
    probe: BrokerSocketProbe,
) -> Result<bool> {
    finish_retirement_probe_with_hook(directory, name, expected, probe, || Ok(()))
}

pub(super) fn finish_retirement_probe_with_hook<F>(
    directory: &SocketDirectory,
    name: &CStr,
    expected: SocketIdentity,
    probe: BrokerSocketProbe,
    before_remove: F,
) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    match probe {
        BrokerSocketProbe::Live => Ok(false),
        BrokerSocketProbe::Refused => {
            before_remove()?;
            directory.remove_expected(name, expected).map(|_| true)
        }
        BrokerSocketProbe::Indeterminate(error) => {
            Err(error).context(BROKER_SOCKET_PROBE_ERROR_CONTEXT)
        }
        BrokerSocketProbe::TimedOut => bail!(BROKER_SOCKET_PROBE_TIMEOUT_CONTEXT),
    }
}

pub(super) fn socket_name(value: &str) -> Result<CString> {
    CString::new(value).context(BROKER_SOCKET_NAME_INVALID)
}
