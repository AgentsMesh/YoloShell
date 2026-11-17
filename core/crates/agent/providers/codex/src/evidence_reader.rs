use super::constants;
use super::evidence_path::AnchoredEvidence;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) fn read_with_hook<F>(
    data_home: &Path,
    path: &Path,
    after_read: F,
) -> Result<Option<Vec<Value>>>
where
    F: FnOnce(),
{
    let Some(mut evidence) = AnchoredEvidence::open(data_home, path)? else {
        return Ok(None);
    };
    let head = read_complete_head(evidence.file_mut())?;
    after_read();
    if !evidence.verify()? {
        return Ok(None);
    }
    Ok(Some(head))
}

pub(super) fn read_complete_head(file: &mut File) -> Result<Vec<Value>> {
    let limit = crate::agent::constants::EVIDENCE_HEAD_BYTES;
    let mut bytes = read_at(file, SeekFrom::Start(constants::FILE_START_OFFSET), limit)?;
    truncate_to_complete_lines(&mut bytes);
    Ok(parse_bytes(&bytes))
}

pub(super) fn read_at(file: &mut File, start: SeekFrom, limit: u64) -> Result<Vec<u8>> {
    file.seek(start).context(constants::EVIDENCE_SEEK_ERROR)?;
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .context(constants::EVIDENCE_READ_ERROR)?;
    Ok(bytes)
}

pub(super) fn parse_bytes(bytes: &[u8]) -> Vec<Value> {
    bytes
        .split(|byte| *byte == constants::JSON_LINE_ENDING_BYTE)
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}

fn truncate_to_complete_lines(bytes: &mut Vec<u8>) {
    let complete = bytes
        .iter()
        .rposition(|byte| *byte == constants::JSON_LINE_ENDING_BYTE)
        .map_or(0, |index| index + 1);
    bytes.truncate(complete);
}
