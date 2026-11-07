use super::delivery_fixture::{CONTENTS_DIRECTORY_ENTRY, PAYLOAD_ENTRY};
use crate::developer_release_constants::{NOTARY_TICKET_ARCHIVE_PATH, SIGNED_ARCHIVE_PATHS};
use anyhow::{Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use yolo_release_archive_audit::constants::{
    DIRECTORY_MODE, EXECUTABLE_MODE, ROOT_DIRECTORY_ENTRY, UNIX_REGULAR_TYPE,
};
use yolo_release_archive_audit_test_support::{FixtureEntry, build};

const MACOS_DIRECTORY_ENTRY: &[u8] = b"Yolo.app/Contents/MacOS/";
const HELPERS_DIRECTORY_ENTRY: &[u8] = b"Yolo.app/Contents/Helpers/";
pub const EXECUTABLE_BYTES: &[u8] = b"fixture-executable";
pub const MUTATED_EXECUTABLE_BYTES: &[u8] = b"mutated-executable";
pub const RESOURCE_BYTES: &[u8] = b"fixture-resource";
const MUTATED_RESOURCE_BYTES: &[u8] = b"mutated-resource";
const TICKET_BYTES: &[u8] = b"notarization-ticket";
const MUTATED_TICKET_BYTES: &[u8] = b"changed-ticket-data";

#[derive(Clone, Copy)]
pub enum DistributedArchiveKind {
    Exact,
    MutatedExecutable,
    MutatedTicket,
}

pub fn candidate_archive() -> Vec<u8> {
    archive(false, EXECUTABLE_BYTES, RESOURCE_BYTES, TICKET_BYTES)
}

pub fn distributed_archive(kind: DistributedArchiveKind) -> Vec<u8> {
    let executable = match kind {
        DistributedArchiveKind::MutatedExecutable => MUTATED_EXECUTABLE_BYTES,
        _ => EXECUTABLE_BYTES,
    };
    let ticket = match kind {
        DistributedArchiveKind::MutatedTicket => MUTATED_TICKET_BYTES,
        _ => TICKET_BYTES,
    };
    archive(true, executable, RESOURCE_BYTES, ticket)
}

pub fn mutated_signed_archive() -> Vec<u8> {
    archive(
        false,
        EXECUTABLE_BYTES,
        MUTATED_RESOURCE_BYTES,
        TICKET_BYTES,
    )
}

pub fn install_executables(contents: &Path) -> Result<()> {
    for path in SIGNED_ARCHIVE_PATHS {
        let relative =
            Path::new(path).strip_prefix(std::str::from_utf8(CONTENTS_DIRECTORY_ENTRY)?)?;
        let executable = contents.join(relative);
        let directory = executable
            .parent()
            .context("fixture executable path has no parent")?;
        fs::create_dir_all(directory)?;
        fs::write(&executable, EXECUTABLE_BYTES)?;
        fs::set_permissions(
            &executable,
            fs::Permissions::from_mode(u32::from(EXECUTABLE_MODE)),
        )?;
        fs::set_permissions(
            directory,
            fs::Permissions::from_mode(u32::from(DIRECTORY_MODE)),
        )?;
    }
    Ok(())
}

pub fn set_mode(path: &Path, mode: u32) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(Into::into)
}

fn archive(with_ticket: bool, executable: &[u8], resource: &[u8], ticket: &[u8]) -> Vec<u8> {
    let mut entries = vec![
        FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
        FixtureEntry::directory(CONTENTS_DIRECTORY_ENTRY),
        FixtureEntry::directory(MACOS_DIRECTORY_ENTRY),
        FixtureEntry::directory(HELPERS_DIRECTORY_ENTRY),
        payload_entry(PAYLOAD_ENTRY, resource),
    ];
    entries.extend(
        SIGNED_ARCHIVE_PATHS
            .iter()
            .map(|path| executable_entry(path.as_bytes(), executable)),
    );
    if with_ticket {
        entries.push(payload_entry(NOTARY_TICKET_ARCHIVE_PATH.as_bytes(), ticket));
    }
    build(&entries).bytes
}

fn executable_entry(path: &[u8], payload: &[u8]) -> FixtureEntry {
    let mut entry = payload_entry(path, payload);
    entry.unix_mode = UNIX_REGULAR_TYPE | EXECUTABLE_MODE;
    entry
}

fn payload_entry(path: &[u8], payload: &[u8]) -> FixtureEntry {
    let mut entry = FixtureEntry::regular(path);
    entry.payload = payload.to_vec();
    entry.crc32 = crc32fast::hash(payload);
    entry.uncompressed_size = u32::try_from(payload.len()).unwrap();
    entry
}
