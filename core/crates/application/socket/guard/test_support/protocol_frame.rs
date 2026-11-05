use super::super::constants::*;
use anyhow::{Result, ensure};
use std::io::{BufRead, Write};

pub(super) fn write_fields(writer: &mut impl Write, fields: &[&str]) -> Result<()> {
    ensure!(
        fields
            .iter()
            .all(|field| { !field.contains(FRAME_SEPARATOR) && !field.contains(FRAME_TERMINATOR) }),
        WATCHDOG_PROTOCOL_DETAIL
    );
    let separator = FRAME_SEPARATOR.to_string();
    writer.write_all(fields.join(&separator).as_bytes())?;
    writer.write_all(FRAME_TERMINATOR.to_string().as_bytes())?;
    writer.flush()?;
    Ok(())
}

pub(super) fn read(reader: &mut impl BufRead) -> Result<String> {
    let mut value = String::new();
    let mut limited = std::io::Read::take(
        reader,
        MAXIMUM_CONTROL_FRAME_BYTES + CONTROL_FRAME_OVERFLOW_SENTINEL_BYTES,
    );
    let read = limited.read_line(&mut value)?;
    validate(&value, read)?;
    Ok(value)
}

pub(super) fn parse(value: &str) -> Result<Vec<&str>> {
    validate(value, value.len())?;
    let frame = value
        .strip_suffix(FRAME_TERMINATOR)
        .ok_or_else(|| anyhow::anyhow!(WATCHDOG_PROTOCOL_DETAIL))?;
    Ok(frame.split(FRAME_SEPARATOR).collect())
}

fn validate(value: &str, bytes: usize) -> Result<()> {
    ensure!(
        bytes > usize::default()
            && u64::try_from(bytes)? <= MAXIMUM_CONTROL_FRAME_BYTES
            && value.ends_with(FRAME_TERMINATOR),
        WATCHDOG_PROTOCOL_DETAIL
    );
    Ok(())
}
