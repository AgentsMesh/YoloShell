use std::path::{Path, PathBuf};

use yolo_trusted_plugin_manifest::{TrustedPluginManifest, load};

use crate::{RegistryPaths, RegistryRecord, RegistryError, source_digest};

impl super::Registry {
    pub fn link(
        &mut self,
        source: &Path,
        paths: &RegistryPaths,
    ) -> Result<RegistryRecord, RegistryError> {
        self.link_checked(source, None, paths)
    }

    /// Links only the digest approved in the pre-link preview.
    pub fn link_approved(
        &mut self,
        source: &Path,
        digest: &str,
        paths: &RegistryPaths,
    ) -> Result<RegistryRecord, RegistryError> {
        self.link_checked(source, Some(digest), paths)
    }

    /// Replaces an existing link for the same id after the approved preview.
    pub fn replace_approved(
        &mut self,
        source: &Path,
        digest: &str,
        paths: &RegistryPaths,
    ) -> Result<RegistryRecord, RegistryError> {
        let record = prepared(source, Some(digest))?;
        self.insert(record.clone());
        self.save(paths)?;
        Ok(record)
    }

    fn link_checked(
        &mut self,
        source: &Path,
        approved: Option<&str>,
        paths: &RegistryPaths,
    ) -> Result<RegistryRecord, RegistryError> {
        let record = prepared(source, approved)?;
        if self.get(&record.id).is_some() {
            return Err(RegistryError::Duplicate);
        }
        self.insert(record.clone());
        self.save(paths)?;
        Ok(record)
    }
}

fn prepared(source: &Path, approved: Option<&str>) -> Result<RegistryRecord, RegistryError> {
    let source = source.canonicalize().map_err(|_| RegistryError::Invalid)?;
    let manifest = load(&source)?;
    let record = record(&manifest, source)?;
    if approved.is_some_and(|digest| digest != record.digest) {
        return Err(RegistryError::DigestMismatch);
    }
    Ok(record)
}

fn record(
    manifest: &TrustedPluginManifest,
    source: PathBuf,
) -> Result<RegistryRecord, RegistryError> {
    Ok(RegistryRecord {
        id: manifest.id.clone(),
        digest: source_digest(&source)?,
        source,
        enabled: true,
        platforms: manifest.platforms.clone(),
        installed_at_unix_ms: now_ms(),
        last_startup: None,
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
