use crate::{
    Installation, IntegrationStatus, Preview, configuration,
    constants::*,
    directory::{self, Directory},
    filesystem, publication,
};
use anyhow::{Result, ensure};
use std::path::Path;
use serde_json::Value;
use yolo_hook_installation_contract::HookInstallSpec;

pub fn inspect(spec: &HookInstallSpec, home: &Path) -> Result<Preview> {
    match std::fs::symlink_metadata(home) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Preview {
            provider: spec.provider.into(),
            path: home.join(spec.file_name),
            revision: directory::missing_revision(home, spec.file_name)?,
            status: IntegrationStatus::Missing,
            requires_provider_trust: spec.requires_provider_trust,
        }),
        Err(error) => Err(error.into()),
        Ok(_) => {
            let directory = Directory::open(home)?;
            let snapshot = filesystem::read(&directory, spec.file_name)?;
            let status = if configuration::restricted(spec, &snapshot.document) {
                IntegrationStatus::Restricted
            } else if configuration::merge(spec, &snapshot.document)? == snapshot.document {
                IntegrationStatus::Configured
            } else {
                IntegrationStatus::Missing
            };
            Ok(Preview {
                provider: spec.provider.into(),
                path: home.join(spec.file_name),
                revision: snapshot.revision,
                status,
                requires_provider_trust: spec.requires_provider_trust,
            })
        }
    }
}

/// Whether the provider's existing configuration already contains a managed
/// callback for this provider, in any historical command form. Invalid or
/// missing documents report false; the installer never creates a profile the
/// user did not configure.
pub fn contains_managed_path(spec: &HookInstallSpec, home: &Path) -> Result<bool> {
    let bytes = match std::fs::read(home.join(spec.file_name)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let Ok(document) = serde_json::from_slice::<Value>(&bytes) else {
        return Ok(false);
    };
    Ok(configuration::contains_managed(spec, &document))
}

/// Remove backups written by earlier releases with a provider-scanned
/// extension. They are Yolo-owned files and must not remain active hooks.
pub fn remove_legacy_backups(home: &Path) -> Result<()> {
    let entries = match std::fs::read_dir(home) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with(crate::constants::BACKUP_PREFIX)
            && name.ends_with(crate::constants::LEGACY_BACKUP_SUFFIX)
            && entry.file_type()?.is_file()
        {
            std::fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

pub fn install(
    spec: &HookInstallSpec,
    home: &Path,
    expected_revision: &str,
) -> Result<Installation> {
    let (directory, created) = match std::fs::symlink_metadata(home) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
            directory::create_missing(home, spec.file_name, expected_revision)?,
            true,
        ),
        Err(error) => return Err(error.into()),
        Ok(_) => (Directory::open(home)?, false),
    };
    let target = spec.file_name;
    remove_legacy_backups(directory.path())?;
    let snapshot = filesystem::read(&directory, target)?;
    ensure!(
        (created && snapshot.identity.is_none()) || snapshot.revision == expected_revision,
        CONFIG_CHANGED
    );
    let merged = configuration::merge(spec, &snapshot.document)?;
    if merged == snapshot.document {
        return Ok(Installation {
            preview: inspect(spec, home)?,
            changed: false,
            backup: None,
        });
    }
    let backup = publication::backup(&directory, target, &snapshot)?;
    publication::write(
        &directory,
        target,
        &snapshot,
        &serde_json::to_vec_pretty(&merged)?,
    )?;
    Ok(Installation {
        preview: inspect(spec, home)?,
        changed: true,
        backup,
    })
}
