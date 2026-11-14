use crate::delivery_files::ProtectedFile;
use crate::developer_release_command::DeveloperReleaseCommandRunner;
use anyhow::{Result, ensure};
use yolo_release_archive_audit::audit_opened_archive;
use yolo_release_archive_audit::model::ZipInventory;
use yolo_release_contracts::ARCHIVE_NAME;
use yolo_release_contracts::{DELIVERY_ARCHIVE_DIGEST_ERROR, DELIVERY_ARCHIVE_INVENTORY_ERROR};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::opened_file_sha256;

pub struct DeveloperReleaseArchive {
    file: ProtectedFile,
    digest: String,
    size: u64,
    inventory: ZipInventory,
}

impl DeveloperReleaseArchive {
    pub fn create(
        root: &DirectoryHandle,
        runner: &dyn DeveloperReleaseCommandRunner,
    ) -> Result<Self> {
        runner.create_archive(root, yolo_release_contracts::APP_NAME, ARCHIVE_NAME)?;
        let file = ProtectedFile::seal_existing(root, ARCHIVE_NAME)?;
        let size = file.file().metadata()?.len();
        let digest = opened_file_sha256(file.file())?;
        let inventory = audit_opened_archive(file.file())?;
        Ok(Self {
            file,
            digest,
            size,
            inventory,
        })
    }

    pub fn open(root: &DirectoryHandle) -> Result<Self> {
        let file = ProtectedFile::open(root, ARCHIVE_NAME)?;
        let size = file.file().metadata()?.len();
        let digest = opened_file_sha256(file.file())?;
        let inventory = audit_opened_archive(file.file())?;
        Ok(Self {
            file,
            digest,
            size,
            inventory,
        })
    }

    pub fn verify_at(&self, root: &DirectoryHandle) -> Result<()> {
        let reopened = self.file.verify_at(root, ARCHIVE_NAME)?;
        ensure!(
            opened_file_sha256(&reopened)? == self.digest,
            DELIVERY_ARCHIVE_DIGEST_ERROR
        );
        ensure!(
            audit_opened_archive(&reopened)? == self.inventory,
            DELIVERY_ARCHIVE_INVENTORY_ERROR
        );
        Ok(())
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn inventory(&self) -> &ZipInventory {
        &self.inventory
    }
}
