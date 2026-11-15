use crate::{
    InstalledBundle, ReleaseStore,
    checkpoint::{self, Checkpoint},
    constants::*,
    inventory,
};
use anyhow::{Result, ensure};
use std::{io::ErrorKind, path::Path};
use yolo_distribution_bundle::RELEASES_DIRECTORY;
use yolo_distribution_publication_protocol::validate_install_root;
use yolo_paths::{
    directory::PrivateDirectory, namespace_lease::NamespaceLease, private_file::io_kind,
};

impl ReleaseStore {
    /// Opens existing private directories without creating paths, changing permissions,
    /// acquiring a coordination lock, or recovering any pending filesystem mutation.
    pub fn open_existing(path: &Path) -> Result<Option<Self>> {
        validate_install_root(path)?;
        let original = match PrivateDirectory::open(path) {
            Ok(directory) => directory,
            Err(error) if io_kind(&error) == Some(ErrorKind::NotFound) => return Ok(None),
            Err(error) => return Err(error),
        };
        let canonical = std::fs::canonicalize(path)?;
        let root = PrivateDirectory::open(&canonical)?;
        ensure!(root.same_identity(&original), IDENTITY_CHANGED);
        original.verify_path()?;
        if !inventory::exists(&root, RELEASES_DIRECTORY)? {
            let namespace = NamespaceLease::from_directories(vec![(ROOT_NAMESPACE, root)])?;
            let selector = canonical.join(CURRENT_FILE_NAME);
            ensure!(namespace.file_is_missing(&selector)?, RELEASES_MISSING);
            namespace.verify()?;
            ensure!(namespace.file_is_missing(&selector)?, RELEASES_MISSING);
            return Ok(None);
        }
        let releases = root.open_descendant(&canonical.join(RELEASES_DIRECTORY))?;
        let namespace = NamespaceLease::from_directories(vec![
            (ROOT_NAMESPACE, root.try_clone()?),
            (RELEASES_NAMESPACE, releases.try_clone()?),
        ])?;
        Ok(Some(Self {
            root,
            releases,
            namespace,
        }))
    }

    /// A read-only observation: concurrent selector changes are reported as conflicts,
    /// including replacements whose JSON value is identical but file identity changed.
    pub fn inspect_existing(&self) -> Result<Option<InstalledBundle>> {
        self.namespace.verify()?;
        let current = self.read_selection()?;
        let installed = if let Some((selection, _)) = &current {
            let release = self.selected_release(selection)?;
            release.verify()?;
            Some(InstalledBundle {
                selection: selection.clone(),
                directory: release.directory.path().to_owned(),
                descriptor: release.descriptor,
            })
        } else {
            None
        };
        checkpoint::reach(Checkpoint::InspectionVerified)?;
        self.verify_selection(&current)?;
        self.namespace.verify()?;
        Ok(installed)
    }
}
