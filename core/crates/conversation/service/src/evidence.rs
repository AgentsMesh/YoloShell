use anyhow::Result;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use yolo_conversation_protocol::ConversationError;
use yolo_crypto::Sha256Digest;
use yolo_paths::identity::ObjectIdentity;

pub(super) fn object(file: &File) -> Result<Sha256Digest> {
    let object = ObjectIdentity::from_file(file)?.ok_or(ConversationError::EvidenceUnavailable)?;
    Sha256Digest::capture(&object)
}

pub(super) fn reader(file: &File) -> Result<impl Read> {
    let mut copy = file.try_clone()?;
    copy.seek(SeekFrom::Start(0))?;
    Ok(copy)
}

pub(super) fn stamp(file: &File) -> Result<Sha256Digest> {
    let metadata = file.metadata()?;
    Sha256Digest::capture(&(
        object(file)?,
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    ))
}

pub(super) fn snapshot_stamp(file: &File, length: u64) -> Result<Option<Sha256Digest>> {
    let before = stamp(file)?;
    Ok((file.metadata()?.len() == length && stamp(file)? == before).then_some(before))
}
