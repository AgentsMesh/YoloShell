use super::super::constants::*;
use super::{ControlCapability, frame};
use anyhow::{Result, ensure};
use std::io::{BufRead, Write};
use yolo_process::ProcessIdentity;

pub(in crate::socket_root_guard) fn write_tracked_root(
    writer: &mut impl Write,
    capability: &ControlCapability,
    identity: ProcessIdentity,
    descriptor: u64,
    origin: ProcessIdentity,
) -> Result<()> {
    ensure!(descriptor != u64::default(), WATCHDOG_PROTOCOL_DETAIL);
    frame::write_fields(
        writer,
        &[
            FRAME_TRACK,
            capability.value(),
            &identity.pid.to_string(),
            &identity.start_time.to_string(),
            &descriptor.to_string(),
            &origin.pid.to_string(),
            &origin.start_time.to_string(),
        ],
    )
}

pub(in crate::socket_root_guard) fn read_tracked_root(
    reader: &mut impl BufRead,
    capability: &ControlCapability,
) -> Result<(ProcessIdentity, u64, ProcessIdentity)> {
    let value = frame::read(reader)?;
    let fields = frame::parse(&value)?;
    let [
        kind,
        actual,
        pid,
        start_time,
        descriptor,
        origin_pid,
        origin_start,
    ] = fields.as_slice()
    else {
        anyhow::bail!(WATCHDOG_PROTOCOL_DETAIL);
    };
    ensure!(
        *kind == FRAME_TRACK && capability.matches(actual),
        WATCHDOG_PROTOCOL_DETAIL
    );
    let identity = ProcessIdentity {
        pid: pid.parse()?,
        start_time: start_time.parse()?,
    };
    let descriptor = descriptor.parse::<u64>()?;
    let origin = ProcessIdentity {
        pid: origin_pid.parse()?,
        start_time: origin_start.parse()?,
    };
    ensure!(identity.pid != u32::default(), WATCHDOG_PROTOCOL_DETAIL);
    ensure!(
        identity.start_time != u64::default(),
        WATCHDOG_PROTOCOL_DETAIL
    );
    ensure!(descriptor != u64::default(), WATCHDOG_PROTOCOL_DETAIL);
    ensure!(
        origin.pid != u32::default() && origin.start_time != u64::default(),
        WATCHDOG_PROTOCOL_DETAIL
    );
    Ok((identity, descriptor, origin))
}
