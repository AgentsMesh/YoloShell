use super::{diagnostic_constants::READ_LABEL, *};
use crate::host::tests::*;

#[derive(Debug)]
pub(super) enum Presence {
    Missing,
    Regular,
    Other,
    MetadataError(std::io::ErrorKind),
}

#[derive(Debug)]
pub(super) enum Reading<T> {
    Missing,
    Present(T),
    InvalidJson,
    InvalidRecord,
    MismatchedBinding,
    IoError(std::io::ErrorKind),
    Rejected,
}

#[derive(Debug)]
pub(super) struct Evidence<T> {
    presence: Presence,
    reading: Reading<T>,
}

/// Uses bounded stable private-file reads and never reconciles or cleans state.
pub(super) fn read<T: serde::de::DeserializeOwned, U>(
    path: &Path,
    max_bytes: u64,
    project: impl FnOnce(T) -> Reading<U>,
) -> Evidence<U> {
    let presence = match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => Presence::Regular,
        Ok(_) => Presence::Other,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Presence::Missing,
        Err(error) => Presence::MetadataError(error.kind()),
    };
    let reading = match read_private_durable_json(path, READ_LABEL, max_bytes) {
        Ok(Some(value)) => project(value),
        Ok(None) => Reading::Missing,
        Err(error) => {
            if error.downcast_ref::<serde_json::Error>().is_some() {
                Reading::InvalidJson
            } else if let Some(error) = error.downcast_ref::<std::io::Error>() {
                Reading::IoError(error.kind())
            } else {
                Reading::Rejected
            }
        }
    };
    Evidence { presence, reading }
}
