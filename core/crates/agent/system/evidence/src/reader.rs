use crate::anchored::AnchoredEvidence;
use crate::constants::{EVIDENCE_HEAD_BYTES, EVIDENCE_TAIL_BYTES, message};
use crate::layout::EvidenceLayout;
use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub struct EvidenceWindow {
    pub head: Vec<Value>,
    pub values: Vec<Value>,
}

pub fn read_with_hook<F>(
    data_home: &Path,
    path: &Path,
    layout: EvidenceLayout,
    after_read: F,
) -> Result<Option<EvidenceWindow>>
where
    F: FnOnce(),
{
    let Some(mut evidence) = AnchoredEvidence::open(data_home, path, layout)? else {
        return Ok(None);
    };
    let window = read_anchored(&mut evidence)?;
    after_read();
    if !evidence.verify()? {
        return Ok(None);
    }
    Ok(Some(window))
}

pub fn read_anchored(evidence: &mut AnchoredEvidence) -> Result<EvidenceWindow> {
    let identity = evidence.identity();
    let head = read_complete_head(evidence.file_mut(), identity.length)?;
    let values = read_window(evidence.file_mut(), identity.length)?;
    Ok(EvidenceWindow { head, values })
}

fn read_complete_head(file: &mut File, length: u64) -> Result<Vec<Value>> {
    let mut bytes = read_at(file, SeekFrom::Start(0), EVIDENCE_HEAD_BYTES)?;
    if length > EVIDENCE_HEAD_BYTES {
        let complete = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1);
        bytes.truncate(complete);
    }
    Ok(parse_bytes(&bytes))
}

fn read_window(file: &mut File, length: u64) -> Result<Vec<Value>> {
    if length <= EVIDENCE_HEAD_BYTES.saturating_add(EVIDENCE_TAIL_BYTES) {
        return read_at(file, SeekFrom::Start(0), length).map(|bytes| parse_bytes(&bytes));
    }
    let head = read_at(file, SeekFrom::Start(0), EVIDENCE_HEAD_BYTES)?;
    let mut values = parse_bytes(&head);
    let tail = read_at(
        file,
        SeekFrom::Start(length.saturating_sub(EVIDENCE_TAIL_BYTES)),
        EVIDENCE_TAIL_BYTES,
    )?;
    let complete_tail = tail
        .iter()
        .position(|byte| *byte == b'\n')
        .map_or(&[][..], |index| &tail[index + 1..]);
    values.extend(parse_bytes(complete_tail));
    Ok(values)
}

pub fn read_at(file: &mut File, start: SeekFrom, limit: u64) -> Result<Vec<u8>> {
    file.seek(start).context(message::EVIDENCE_SEEK)?;
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .context(message::EVIDENCE_READ)?;
    Ok(bytes)
}

pub fn parse_bytes(bytes: &[u8]) -> Vec<Value> {
    bytes
        .split(|byte| *byte == b'\n')
        .filter_map(|line| serde_json::from_slice(line).ok())
        .collect()
}
