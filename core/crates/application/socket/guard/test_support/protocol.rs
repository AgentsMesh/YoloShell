use super::constants::*;
use anyhow::{Context, Result, ensure};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use yolo_process::ProcessIdentity;

#[path = "protocol_capability.rs"]
mod capability;
#[path = "protocol_completion.rs"]
mod completion;
#[path = "protocol_descriptor.rs"]
mod descriptor;
#[path = "protocol_frame.rs"]
mod frame;
pub(super) use capability::{ControlCapability, DiscoveryCapability};
pub(super) use completion::{CompletionFrame, parse_completion, write_completion};
pub(super) use descriptor::{read_tracked_root, write_tracked_root};

pub(super) struct RootFrame {
    pub(super) capability: ControlCapability,
    pub(super) discovery: DiscoveryCapability,
    pub(super) discovery_reader: u64,
    pub(super) discovery_writer: u64,
    pub(super) path: PathBuf,
}

pub(super) fn write_root(
    writer: &mut impl Write,
    capability: &ControlCapability,
    discovery: &DiscoveryCapability,
    discovery_reader: u64,
    discovery_writer: u64,
    path: &Path,
) -> Result<()> {
    let path = path.to_str().context(WATCHDOG_PROTOCOL_DETAIL)?;
    ensure!(
        discovery_reader != u64::default()
            && discovery_writer != u64::default()
            && discovery_reader != discovery_writer,
        WATCHDOG_PROTOCOL_DETAIL
    );
    frame::write_fields(
        writer,
        &[
            FRAME_ROOT,
            capability.value(),
            discovery.value(),
            &discovery_reader.to_string(),
            &discovery_writer.to_string(),
            path,
        ],
    )
}

#[cfg(test)]
pub(super) fn read_root(reader: &mut impl BufRead) -> Result<RootFrame> {
    parse_root(&frame::read(reader)?)
}

pub(super) fn parse_root(value: &str) -> Result<RootFrame> {
    let fields = frame::parse(value)?;
    let [kind, capability, discovery, reader, writer, path] = fields.as_slice() else {
        anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL);
    };
    ensure!(*kind == FRAME_ROOT, WATCHDOG_PROTOCOL_DETAIL);
    let path = PathBuf::from(path);
    ensure!(path.is_absolute(), WATCHDOG_PROTOCOL_DETAIL);
    let discovery_reader = reader.parse::<u64>()?;
    let discovery_writer = writer.parse::<u64>()?;
    ensure!(
        discovery_reader != u64::default()
            && discovery_writer != u64::default()
            && discovery_reader != discovery_writer,
        WATCHDOG_PROTOCOL_DETAIL
    );
    Ok(RootFrame {
        capability: ControlCapability::parse(capability)?,
        discovery: DiscoveryCapability::parse(discovery)?,
        discovery_reader,
        discovery_writer,
        path,
    })
}

pub(super) fn write_identity(
    writer: &mut impl Write,
    kind: &str,
    capability: &ControlCapability,
    identity: ProcessIdentity,
) -> Result<()> {
    frame::write_fields(
        writer,
        &[
            kind,
            capability.value(),
            &identity.pid.to_string(),
            &identity.start_time.to_string(),
        ],
    )
}

#[cfg(test)]
pub(super) fn read_identity(
    reader: &mut impl BufRead,
    kind: &str,
    capability: &ControlCapability,
) -> Result<ProcessIdentity> {
    parse_identity(&frame::read(reader)?, kind, capability)
}

pub(super) fn parse_identity(
    value: &str,
    kind: &str,
    capability: &ControlCapability,
) -> Result<ProcessIdentity> {
    let fields = frame::parse(value)?;
    let [actual_kind, actual_capability, pid, start_time] = fields.as_slice() else {
        anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL);
    };
    ensure!(
        *actual_kind == kind && capability.matches(actual_capability),
        WATCHDOG_PROTOCOL_DETAIL
    );
    let pid = pid.parse::<u32>()?;
    let start_time = start_time.parse::<u64>()?;
    ensure!(
        pid != u32::default() && start_time != u64::default(),
        WATCHDOG_PROTOCOL_DETAIL
    );
    Ok(ProcessIdentity { pid, start_time })
}

pub(super) fn write_capability(
    writer: &mut impl Write,
    kind: &str,
    capability: &ControlCapability,
) -> Result<()> {
    frame::write_fields(writer, &[kind, capability.value()])
}

pub(super) fn read_capability(
    reader: &mut impl BufRead,
    kind: &str,
    capability: &ControlCapability,
) -> Result<()> {
    parse_capability(&frame::read(reader)?, kind, capability)
}

pub(super) fn parse_capability(
    value: &str,
    kind: &str,
    capability: &ControlCapability,
) -> Result<()> {
    let fields = frame::parse(value)?;
    let [actual_kind, actual_capability] = fields.as_slice() else {
        anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL);
    };
    ensure!(
        *actual_kind == kind && capability.matches(actual_capability),
        WATCHDOG_PROTOCOL_DETAIL
    );
    Ok(())
}

pub(super) fn read_new_capability(
    reader: &mut impl BufRead,
    kind: &str,
) -> Result<ControlCapability> {
    let value = frame::read(reader)?;
    let fields = frame::parse(&value)?;
    let [actual_kind, capability] = fields.as_slice() else {
        anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL);
    };
    ensure!(*actual_kind == kind, WATCHDOG_PROTOCOL_DETAIL);
    ControlCapability::parse(capability)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))] #[path = "../unit_tests/protocol_tests.rs"]
mod tests;
