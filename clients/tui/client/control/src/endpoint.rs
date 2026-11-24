use crate::constants::*;
use anyhow::{Result, ensure};
use std::{io::Read, path::Path};
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, private_file::PrivateFileEntry,
};
use yolo_tui_client_control_protocol::ControlDescriptor;

pub(crate) struct DescriptorRead {
    pub entry: PrivateFileEntry,
    pub identity: FileIdentity,
    pub value: ControlDescriptor,
}

impl DescriptorRead {
    pub fn open(root: &Path) -> Result<Self> {
        let root = yolo_paths::platform::platform_path(root);
        let directory = PrivateDirectory::open(&root)?;
        let entry =
            PrivateFileEntry::from_directory(directory, &root.join(CONTROL_DESCRIPTOR_NAME))?;
        let mut opened = entry.open_read(CONTROL_DESCRIPTOR_MAX_BYTES)?;
        let identity = opened.identity();
        let mut bytes = Vec::new();
        opened
            .file_mut()
            .take(CONTROL_DESCRIPTOR_MAX_BYTES + CONTROL_READ_SENTINEL)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() <= MAX_CONTROL_FRAME_BYTES,
            CONTROL_DESCRIPTOR_ERROR
        );
        let value: ControlDescriptor = serde_json::from_slice(&bytes)?;
        ensure!(value.is_valid(), CONTROL_DESCRIPTOR_ERROR);
        entry.verify_current_unchanged(identity, CONTROL_DESCRIPTOR_MAX_BYTES)?;
        Ok(Self {
            entry,
            identity,
            value,
        })
    }

    pub fn verify(&self) -> Result<()> {
        self.entry
            .verify_current_unchanged(self.identity, CONTROL_DESCRIPTOR_MAX_BYTES)
    }
}
