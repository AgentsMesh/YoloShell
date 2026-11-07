use super::secure_vfs_alias_init::PublishedAlias;
use super::secure_vfs_alias_io::{
    ALIAS_ACTIVE_MODE, ALIAS_MAIN_NAME, ALIAS_SEALED_MODE, create_symlink, unlink_name,
};
use super::secure_vfs_file::{ArtifactKind, SecureDescriptor};
use super::secure_vfs_filename::SqliteFilename;
use anyhow::Result;
use std::path::{Path, PathBuf};

const SQLITE_SHM_SUFFIX: &str = "-shm";
pub(super) struct SecureAlias {
    owner: PublishedAlias,
    kind: ArtifactKind,
    main: PathBuf,
    opened: PathBuf,
    sqlite_name: SqliteFilename,
}

impl SecureAlias {
    pub(super) fn create(descriptor: &SecureDescriptor, kind: ArtifactKind) -> Result<Self> {
        let owner = PublishedAlias::create(descriptor, kind)?;
        let main = owner.path().join(ALIAS_MAIN_NAME);
        let opened = append_suffix(&main, kind.suffix());
        let opened_name = opened
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("SQLite alias has no file name"))?;
        descriptor.link_to_directory(owner.directory(), opened_name)?;
        descriptor.verify_alias_link(owner.directory(), opened_name)?;
        let sqlite_name = SqliteFilename::create(&main, kind)?;
        Ok(Self {
            owner,
            kind,
            main,
            opened,
            sqlite_name,
        })
    }

    pub(super) fn sqlite_name(&self) -> *const libc::c_char {
        self.sqlite_name.opened()
    }

    pub(super) fn opened_path(&self) -> &Path {
        &self.opened
    }

    pub(super) fn discard_open_link(&self) {
        self.unlink_opened();
    }

    pub(super) fn release_open_link(&self, descriptor: &SecureDescriptor) -> Result<()> {
        self.unlink_opened();
        if self.kind == ArtifactKind::Main
            && !descriptor.is_retained()
            && let Some(name) = self.opened.file_name()
        {
            create_symlink(self.owner.directory(), name, &descriptor.reference_path())?;
            descriptor.verify_reference(&self.opened)?;
        }
        Ok(())
    }

    pub(super) fn shm_path(&self) -> PathBuf {
        append_suffix(&self.main, SQLITE_SHM_SUFFIX)
    }

    pub(super) fn create_shm_link(&self, descriptor: &SecureDescriptor) -> Result<()> {
        let name = self
            .shm_path()
            .file_name()
            .map(std::ffi::OsStr::to_owned)
            .ok_or_else(|| anyhow::anyhow!("SQLite SHM alias has no file name"))?;
        descriptor.link_to_directory(self.owner.directory(), &name)?;
        descriptor.verify_alias_link(self.owner.directory(), &name)
    }

    pub(super) fn remove_shm_link(&self, descriptor: &SecureDescriptor) -> Result<()> {
        let name = self
            .shm_path()
            .file_name()
            .map(std::ffi::OsStr::to_owned)
            .ok_or_else(|| anyhow::anyhow!("SQLite SHM alias has no file name"))?;
        descriptor.verify_alias_link(self.owner.directory(), &name)?;
        super::secure_vfs_alias_link::remove(self.owner.directory(), &name)
    }

    pub(super) fn seal(&self) -> Result<()> {
        self.owner.set_mode(ALIAS_SEALED_MODE)
    }

    pub(super) fn unseal(&self) -> Result<()> {
        self.owner.set_mode(ALIAS_ACTIVE_MODE)
    }

    fn unlink_opened(&self) {
        if let Some(name) = self.opened.file_name() {
            unlink_name(self.owner.directory(), name);
        }
    }

    #[cfg(test)]
    pub(super) fn directory(&self) -> &Path {
        self.owner.path()
    }
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
