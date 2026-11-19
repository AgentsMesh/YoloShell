use crate::developer_release_constants::*;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;
use yolo_release_archive_audit::constants::REGULAR_RESOURCE_MODE;
use yolo_release_archive_audit::model::{ArchiveEntry, ZipInventory};

pub fn validate_transition(candidate: &ZipInventory, distributed: &ZipInventory) -> Result<()> {
    validate(candidate, distributed, true)
}

pub fn validate_signed_transition(candidate: &ZipInventory, signed: &ZipInventory) -> Result<()> {
    validate(candidate, signed, false)
}

fn validate(candidate: &ZipInventory, distributed: &ZipInventory, stapled: bool) -> Result<()> {
    let before = entries(candidate)?;
    let after = entries(distributed)?;
    ensure!(
        !before.contains_key(NOTARY_TICKET_ARCHIVE_PATH),
        DEVELOPER_RELEASE_INVENTORY_ERROR
    );
    let expected_entries = if stapled {
        before.len().saturating_add(1)
    } else {
        before.len()
    };
    ensure!(
        after.len() == expected_entries,
        DEVELOPER_RELEASE_INVENTORY_ERROR
    );
    for (path, expected) in &before {
        let actual = after
            .get(path)
            .ok_or_else(|| anyhow::anyhow!(DEVELOPER_RELEASE_INVENTORY_ERROR))?;
        if mutable_path(path) {
            ensure!(
                expected.mode == actual.mode && expected.is_directory == actual.is_directory,
                DEVELOPER_RELEASE_INVENTORY_ERROR
            );
        } else {
            ensure!(
                immutable_fields(expected) == immutable_fields(actual),
                DEVELOPER_RELEASE_INVENTORY_ERROR
            );
        }
    }
    if stapled {
        validate_ticket(after.get(NOTARY_TICKET_ARCHIVE_PATH))
    } else {
        ensure!(
            !after.contains_key(NOTARY_TICKET_ARCHIVE_PATH),
            DEVELOPER_RELEASE_INVENTORY_ERROR
        );
        Ok(())
    }
}

fn entries(inventory: &ZipInventory) -> Result<BTreeMap<&str, &ArchiveEntry>> {
    for entry in &inventory.entries {
        if !entry.is_directory
            && EXECUTABLE_ARCHIVE_DIRECTORIES
                .iter()
                .any(|root| entry.path.starts_with(root))
        {
            ensure!(
                SIGNED_ARCHIVE_PATHS.contains(&entry.path.as_str()),
                DEVELOPER_RELEASE_INVENTORY_ERROR
            );
        }
    }
    let entries = inventory
        .entries
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    ensure!(
        entries.len() == inventory.entries.len(),
        DEVELOPER_RELEASE_INVENTORY_ERROR
    );
    Ok(entries)
}

fn mutable_path(path: &str) -> bool {
    path == CODE_RESOURCES_ARCHIVE_PATH || SIGNED_ARCHIVE_PATHS.contains(&path)
}

fn immutable_fields(entry: &ArchiveEntry) -> (u16, bool, u32, &str, u64) {
    (
        entry.mode,
        entry.is_directory,
        entry.crc32,
        entry.sha256.as_str(),
        entry.uncompressed_size,
    )
}

fn validate_ticket(ticket: Option<&&ArchiveEntry>) -> Result<()> {
    let ticket = ticket.ok_or_else(|| anyhow::anyhow!(DEVELOPER_RELEASE_INVENTORY_ERROR))?;
    ensure!(!ticket.is_directory, DEVELOPER_RELEASE_INVENTORY_ERROR);
    ensure!(
        ticket.mode == REGULAR_RESOURCE_MODE,
        DEVELOPER_RELEASE_INVENTORY_ERROR
    );
    ensure!(
        ticket.uncompressed_size > 0 && ticket.uncompressed_size <= MAX_NOTARY_TICKET_BYTES,
        DEVELOPER_RELEASE_INVENTORY_ERROR
    );
    Ok(())
}
