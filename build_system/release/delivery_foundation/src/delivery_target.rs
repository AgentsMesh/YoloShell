use crate::delivery_identity::DeliveryObjectIdentity;
use anyhow::{Context, Result, ensure};
use std::path::Path;
use yolo_release_contracts::{
    DELIVERY_DESTINATION_NAME_ERROR, DELIVERY_DESTINATION_PARENT_ERROR,
    DELIVERY_DESTINATION_UTF8_ERROR,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_filesystem::constants::{CURRENT_DIRECTORY_COMPONENT, PARENT_DIRECTORY_COMPONENT};
use yolo_release_gate_system::{ensure_missing, entry_exists, validate_parent};

pub struct DeliveryTarget {
    pub parent: DirectoryHandle,
    pub destination: String,
}

impl DeliveryTarget {
    pub fn prepare(path: &Path) -> Result<Self> {
        let destination = path
            .file_name()
            .and_then(|value| value.to_str())
            .context(DELIVERY_DESTINATION_UTF8_ERROR)?;
        ensure!(
            !destination.is_empty()
                && destination != CURRENT_DIRECTORY_COMPONENT
                && destination != PARENT_DIRECTORY_COMPONENT,
            DELIVERY_DESTINATION_NAME_ERROR
        );
        let parent_path = path.parent().context(DELIVERY_DESTINATION_PARENT_ERROR)?;
        let parent = DirectoryHandle::open(parent_path)?;
        validate_parent(&parent)?;
        // Reject unsupported filesystems before staging, signing or notarization.
        DeliveryObjectIdentity::capture(&parent)?;
        Ok(Self {
            parent,
            destination: destination.to_owned(),
        })
    }

    pub fn revalidate_missing(&self) -> Result<()> {
        validate_parent(&self.parent)?;
        ensure_missing(&self.parent, &self.destination)
    }

    pub fn existing(&self) -> Result<Option<DirectoryHandle>> {
        validate_parent(&self.parent)?;
        if !entry_exists(&self.parent, &self.destination)? {
            return Ok(None);
        }
        self.open().map(Some)
    }

    pub fn open(&self) -> Result<DirectoryHandle> {
        let delivered = DirectoryHandle::open_at(&self.parent, &self.destination)?;
        self.parent.verify_child(&self.destination, &delivered)?;
        Ok(delivered)
    }
}
