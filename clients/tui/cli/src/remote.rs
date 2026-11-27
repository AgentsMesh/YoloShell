use crate::constants::*;
use anyhow::{Result, ensure};
use std::{io::Read, path::Path, sync::Arc};
use yolo_hub_client::HubEndpoint;
use yolo_hub_ssh_entry::{EntryProfile, MAX_PROFILE_BYTES, SshEntry};
use yolo_paths::private_file::PrivateFileEntry;
use yolo_tui_app::Options;
use zeroize::Zeroizing;

pub(crate) fn options(profile: &Path, account_key: &Path, client_key: &Path) -> Result<Options> {
    let document = read_private(profile, MAX_PROFILE_BYTES as u64)?;
    let profile =
        EntryProfile::decode(&document).map_err(|_| anyhow::anyhow!(INVALID_REMOTE_PROFILE))?;
    let hub = profile.hub.hub_id;
    let entry = SshEntry::new(profile, key(account_key)?, key(client_key)?)
        .map_err(|_| anyhow::anyhow!(INVALID_REMOTE_CREDENTIAL))?;
    let endpoint = HubEndpoint::remote(Arc::new(entry));
    endpoint.validate()?;
    Ok(Options {
        endpoint,
        hub,
        layout: None,
    })
}

fn key(path: &Path) -> Result<Arc<russh::keys::PrivateKey>> {
    let bytes = read_private(path, MAX_PRIVATE_KEY_BYTES)?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| anyhow::anyhow!(INVALID_REMOTE_CREDENTIAL))?;
    let key = russh::keys::decode_secret_key(text, None)
        .map_err(|_| anyhow::anyhow!(INVALID_REMOTE_CREDENTIAL))?;
    Ok(Arc::new(key))
}

fn read_private(path: &Path, limit: u64) -> Result<Zeroizing<Vec<u8>>> {
    yolo_paths::validation::validate_absolute(path)?;
    let entry =
        PrivateFileEntry::open(path).map_err(|_| anyhow::anyhow!(PRIVATE_REMOTE_FILE_REQUIRED))?;
    let mut opened = entry
        .open_read(limit)
        .map_err(|_| anyhow::anyhow!(PRIVATE_REMOTE_FILE_REQUIRED))?;
    let identity = opened.identity();
    let mut bytes = Zeroizing::new(Vec::new());
    opened
        .file_mut()
        .take(limit + READ_LIMIT_SENTINEL)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow::anyhow!(PRIVATE_REMOTE_FILE_REQUIRED))?;
    ensure!(bytes.len() as u64 <= limit, PRIVATE_REMOTE_FILE_REQUIRED);
    entry
        .verify_current_unchanged(identity, limit)
        .map_err(|_| anyhow::anyhow!(PRIVATE_REMOTE_FILE_REQUIRED))?;
    Ok(bytes)
}
