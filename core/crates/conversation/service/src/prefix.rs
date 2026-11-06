use crate::evidence;
use anyhow::Result;
use std::{fs::File, os::unix::fs::FileExt};
use yolo_conversation_protocol::ConversationError;
use yolo_conversation_transcript::{MAX_HISTORY_SCAN_BYTES, SCAN_CHUNK_BYTES};
use yolo_crypto::Sha256Hasher;

pub(super) fn guard<T>(
    file: &File,
    length: u64,
    read: impl FnOnce(&File, u64) -> Result<(T, Option<String>)>,
) -> Result<T> {
    let before = evidence::stamp(file)?;
    let (result, digest) = read(file, length)?;
    if let Some(digest) = digest
        && evidence::stamp(file)? != before
        && hash(file, length)? != digest
    {
        return Err(ConversationError::EvidenceChanged.into());
    }
    Ok(result)
}

pub(super) fn hash(file: &File, length: u64) -> Result<String> {
    if length > MAX_HISTORY_SCAN_BYTES {
        return Err(ConversationError::HistoryLimitExceeded.into());
    }
    let mut digest = Sha256Hasher::new();
    let mut chunk = [0; SCAN_CHUNK_BYTES];
    let mut offset = 0;
    while offset < length {
        let wanted = (length - offset).min(SCAN_CHUNK_BYTES as u64) as usize;
        let count = file.read_at(&mut chunk[..wanted], offset)?;
        if count == 0 {
            return Err(ConversationError::EvidenceChanged.into());
        }
        digest.update(&chunk[..count]);
        offset += count as u64;
    }
    Ok(digest.finish_hex())
}
