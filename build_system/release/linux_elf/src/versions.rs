use yolo_release_contracts::{GLIBC_SYMBOL_VERSION_PREFIX, LINUX_MINIMUM_GLIBC_VERSION};
use crate::reader::Reader;
use anyhow::{Result, ensure};
use std::cmp::Ordering;

const VERSION_NEED_BYTES: usize = 16;
const VERSION_NEED_REVISION_OFFSET: usize = 0;
const VERSION_NEED_COUNT_OFFSET: usize = 2;
const VERSION_NEED_AUXILIARY_OFFSET: usize = 8;
const VERSION_NEED_NEXT_OFFSET: usize = 12;
const VERSION_NEED_CURRENT_REVISION: u16 = 1;
const VERSION_AUXILIARY_BYTES: usize = 16;
const VERSION_AUXILIARY_NAME_OFFSET: usize = 8;
const VERSION_AUXILIARY_NEXT_OFFSET: usize = 12;
const STRING_TERMINATOR: u8 = 0;
const MAX_VERSION_NEED_COUNT: usize = 1_024;
const MAX_VERSION_AUXILIARY_COUNT: usize = 4_096;
const MAX_VERSION_COMPONENT_COUNT: usize = 4;
const MIN_VERSION_COMPONENT_COUNT: usize = 2;
const MISSING_VERSION_COMPONENT: u32 = 0;
const VERSION_NEED_ERROR: &str = "Linux runtime ELF version needs are invalid";
const VERSION_NAME_ERROR: &str = "Linux runtime GLIBC version name is invalid";
const VERSION_REQUIREMENT_ERROR: &str = "Linux runtime requires glibc newer than supported";
const GLIBC_REQUIREMENT_MISSING_ERROR: &str = "Linux runtime has no GLIBC version requirement";

pub(crate) fn verify(bytes: &[u8], count: usize, strings: &[u8]) -> Result<()> {
    ensure!(
        count > 0 && count <= MAX_VERSION_NEED_COUNT,
        VERSION_NEED_ERROR
    );
    let mut cursor = 0;
    let mut total_auxiliaries: usize = 0;
    let mut found_glibc = false;
    for index in 0..count {
        let need = Reader::range(bytes, cursor, VERSION_NEED_BYTES)?;
        ensure!(
            Reader::u16(need, VERSION_NEED_REVISION_OFFSET)? == VERSION_NEED_CURRENT_REVISION,
            VERSION_NEED_ERROR
        );
        let auxiliary_count = usize::from(Reader::u16(need, VERSION_NEED_COUNT_OFFSET)?);
        total_auxiliaries = total_auxiliaries
            .checked_add(auxiliary_count)
            .ok_or_else(|| anyhow::anyhow!(VERSION_NEED_ERROR))?;
        ensure!(
            auxiliary_count > 0 && total_auxiliaries <= MAX_VERSION_AUXILIARY_COUNT,
            VERSION_NEED_ERROR
        );
        let auxiliary_delta = usize::try_from(Reader::u32(need, VERSION_NEED_AUXILIARY_OFFSET)?)?;
        let auxiliary = cursor
            .checked_add(auxiliary_delta)
            .ok_or_else(|| anyhow::anyhow!(VERSION_NEED_ERROR))?;
        found_glibc |= verify_auxiliaries(bytes, auxiliary, auxiliary_count, strings)?;
        cursor = next_record(
            cursor,
            usize::try_from(Reader::u32(need, VERSION_NEED_NEXT_OFFSET)?)?,
            index + 1 == count,
            VERSION_NEED_BYTES,
        )?;
    }
    ensure!(found_glibc, GLIBC_REQUIREMENT_MISSING_ERROR);
    Ok(())
}

fn verify_auxiliaries(
    bytes: &[u8],
    mut cursor: usize,
    count: usize,
    strings: &[u8],
) -> Result<bool> {
    let mut found_glibc = false;
    for index in 0..count {
        let auxiliary = Reader::range(bytes, cursor, VERSION_AUXILIARY_BYTES)?;
        let name_offset = usize::try_from(Reader::u32(auxiliary, VERSION_AUXILIARY_NAME_OFFSET)?)?;
        found_glibc |= verify_name(c_string(strings, name_offset)?)?;
        cursor = next_record(
            cursor,
            usize::try_from(Reader::u32(auxiliary, VERSION_AUXILIARY_NEXT_OFFSET)?)?,
            index + 1 == count,
            VERSION_AUXILIARY_BYTES,
        )?;
    }
    Ok(found_glibc)
}

fn next_record(current: usize, delta: usize, last: bool, minimum: usize) -> Result<usize> {
    if last {
        ensure!(delta == 0, VERSION_NEED_ERROR);
        return Ok(current);
    }
    ensure!(delta >= minimum, VERSION_NEED_ERROR);
    current
        .checked_add(delta)
        .ok_or_else(|| anyhow::anyhow!(VERSION_NEED_ERROR))
}

fn c_string(bytes: &[u8], offset: usize) -> Result<&[u8]> {
    let tail = bytes
        .get(offset..)
        .ok_or_else(|| anyhow::anyhow!(VERSION_NEED_ERROR))?;
    let length = tail
        .iter()
        .position(|byte| *byte == STRING_TERMINATOR)
        .ok_or_else(|| anyhow::anyhow!(VERSION_NEED_ERROR))?;
    Ok(&tail[..length])
}

fn verify_name(name: &[u8]) -> Result<bool> {
    let prefix = GLIBC_SYMBOL_VERSION_PREFIX.as_bytes();
    if !name.starts_with(prefix) {
        return Ok(false);
    }
    let version = std::str::from_utf8(&name[prefix.len()..])?;
    let actual = parse_version(version)?;
    let supported = parse_version(LINUX_MINIMUM_GLIBC_VERSION)?;
    ensure!(
        !version_exceeds(&actual, &supported),
        VERSION_REQUIREMENT_ERROR
    );
    Ok(true)
}

fn parse_version(value: &str) -> Result<Vec<u32>> {
    let components = value
        .split('.')
        .map(str::parse::<u32>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| anyhow::anyhow!(VERSION_NAME_ERROR))?;
    ensure!(
        (MIN_VERSION_COMPONENT_COUNT..=MAX_VERSION_COMPONENT_COUNT).contains(&components.len()),
        VERSION_NAME_ERROR
    );
    Ok(components)
}

fn version_exceeds(actual: &[u32], supported: &[u32]) -> bool {
    let count = actual.len().max(supported.len());
    for index in 0..count {
        let actual = actual
            .get(index)
            .copied()
            .unwrap_or(MISSING_VERSION_COMPONENT);
        let supported = supported
            .get(index)
            .copied()
            .unwrap_or(MISSING_VERSION_COMPONENT);
        match actual.cmp(&supported) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => {}
        }
    }
    false
}
