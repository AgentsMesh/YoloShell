use super::constants;
use super::evidence_path::AnchoredEvidence;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub(super) struct EvidenceWindow {
    pub head: Vec<Value>,
    pub values: Vec<Value>,
}

pub(super) fn read_with_hook<F>(
    data_home: &Path,
    path: &Path,
    after_read: F,
) -> Result<Option<EvidenceWindow>>
where
    F: FnOnce(),
{
    let Some(mut evidence) = AnchoredEvidence::open(data_home, path)? else {
        return Ok(None);
    };
    let window = read_anchored(&mut evidence)?;
    after_read();
    if !evidence.verify()? {
        return Ok(None);
    }
    Ok(Some(window))
}

pub(super) fn read_anchored(evidence: &mut AnchoredEvidence) -> Result<EvidenceWindow> {
    let identity = evidence.identity();
    let head = read_complete_head(evidence.file_mut(), identity.length)?;
    let values = read_window(evidence.file_mut(), identity.length)?;
    Ok(EvidenceWindow { head, values })
}

fn read_complete_head(file: &mut File, length: u64) -> Result<Vec<Value>> {
    let limit = crate::agent::constants::EVIDENCE_HEAD_BYTES;
    let mut bytes = read_at(file, SeekFrom::Start(0), limit)?;
    if length > limit {
        let complete = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        bytes.truncate(complete);
    }
    Ok(parse_bytes(&bytes))
}

fn read_window(file: &mut File, length: u64) -> Result<Vec<Value>> {
    let head_limit = crate::agent::constants::EVIDENCE_HEAD_BYTES;
    let tail_limit = crate::agent::constants::EVIDENCE_TAIL_BYTES;
    if length <= head_limit.saturating_add(tail_limit) {
        return read_at(file, SeekFrom::Start(0), length).map(|bytes| parse_bytes(&bytes));
    }
    let head = read_at(file, SeekFrom::Start(0), head_limit)?;
    let mut values = parse_bytes(&head);
    let tail = read_at(
        file,
        SeekFrom::Start(length.saturating_sub(tail_limit)),
        tail_limit,
    )?;
    let complete_tail = tail
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(&[][..], |index| &tail[index + 1..]);
    values.extend(parse_bytes(complete_tail));
    Ok(values)
}

fn read_at(file: &mut File, start: SeekFrom, limit: u64) -> Result<Vec<u8>> {
    file.seek(start).context(constants::EVIDENCE_SEEK_ERROR)?;
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .context(constants::EVIDENCE_READ_ERROR)?;
    Ok(bytes)
}

fn parse_bytes(bytes: &[u8]) -> Vec<Value> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}
