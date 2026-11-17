use super::constants::*;
use crate::namespace_lease::NamespaceLease;
use crate::paths::Paths;
use crate::paths::constants::{SESSION_LIFECYCLE_LOCK_FILE_NAME, SESSION_LIFECYCLE_LOCK_MAX_BYTES};
use crate::paths::identity::{DurableObjectIdentity, FileIdentity, ObjectIdentity};
use crate::paths::private_file::PrivateDirectoryLock;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::{Context, Result, ensure};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use uuid::Uuid;

pub struct SessionDurableMutationGuard<'a> {
    terminal_id: Uuid,
    namespace: &'a NamespaceLease,
    session_directory: crate::paths::directory::PrivateDirectory,
    session_identity: ObjectIdentity,
    durable_identity: OnceLock<DurableObjectIdentity>,
    intent_path: PathBuf,
    preparation_authority_path: PathBuf,
    _directory_lock: PrivateDirectoryLock,
    _lifecycle_lock: Option<PrivateFileLock>,
}

include!("guard_directory_authority.rs");
include!("guard_durable_identity.rs");
include!("guard_read_versions.rs");

impl<'a> SessionDurableMutationGuard<'a> {
    pub fn acquire(paths: &Paths, namespace: &'a NamespaceLease, terminal_id: Uuid) -> Result<Self> {
        ensure!(!terminal_id.is_nil(), SESSION_ID_INVALID);
        let session_path = paths.session_dir(terminal_id);
        let session_directory = namespace.directory(&session_path)?;
        session_directory.verify_path()?;
        let lifecycle_path = session_path.join(SESSION_LIFECYCLE_LOCK_FILE_NAME);
        let lifecycle_lock = PrivateFileLock::try_acquire_in(
            session_directory.try_clone()?,
            &lifecycle_path,
            SESSION_LIFECYCLE_LOCK_MAX_BYTES,
        )?
        .context(SESSION_LIFECYCLE_BUSY)?;
        Self::acquire_directory(
            paths,
            namespace,
            terminal_id,
            session_directory,
            Some(lifecycle_lock),
        )
    }

    pub fn acquire_participant(
        paths: &Paths,
        namespace: &'a NamespaceLease,
        terminal_id: Uuid,
    ) -> Result<Self> {
        ensure!(!terminal_id.is_nil(), SESSION_ID_INVALID);
        let session_path = paths.session_dir(terminal_id);
        let session_directory = namespace.directory(&session_path)?;
        session_directory.verify_path()?;
        let lifecycle_path = session_path.join(SESSION_LIFECYCLE_LOCK_FILE_NAME);
        let lifecycle_lock = PrivateFileLock::acquire_in_wait(
            session_directory.try_clone()?,
            &lifecycle_path,
            SESSION_LIFECYCLE_LOCK_MAX_BYTES,
            SESSION_PARTICIPANT_LIFECYCLE_LOCK_TIMEOUT,
            SESSION_PARTICIPANT_LIFECYCLE_LOCK_RETRY,
        )
        .context(SESSION_LIFECYCLE_BUSY)?;
        Self::acquire_directory(
            paths,
            namespace,
            terminal_id,
            session_directory,
            Some(lifecycle_lock),
        )
    }

    pub fn acquire_after_lifecycle_lock(
        paths: &Paths,
        namespace: &'a NamespaceLease,
        terminal_id: Uuid,
    ) -> Result<Self> {
        ensure!(!terminal_id.is_nil(), SESSION_ID_INVALID);
        let session_directory = namespace.directory(&paths.session_dir(terminal_id))?;
        Self::acquire_directory(paths, namespace, terminal_id, session_directory, None)
    }

    fn acquire_directory(
        paths: &Paths,
        namespace: &'a NamespaceLease,
        terminal_id: Uuid,
        session_directory: crate::paths::directory::PrivateDirectory,
        lifecycle_lock: Option<PrivateFileLock>,
    ) -> Result<Self> {
        session_directory.verify_path()?;
        let intent_path = paths.reboot_migration_intent(terminal_id);
        let preparation_authority_path = paths.reboot_preparation_authority(terminal_id);
        let intent_entry = namespace.file(&intent_path)?;
        let directory_lock = intent_entry.lock_parent_exclusive_wait(
            SESSION_DIRECTORY_LOCK_TIMEOUT,
            SESSION_DIRECTORY_LOCK_RETRY,
        )?;
        session_directory.verify_path()?;
        let session_identity = session_directory.object_identity()?;
        namespace.verify()?;
        Ok(Self {
            terminal_id,
            namespace,
            session_directory,
            session_identity,
            durable_identity: OnceLock::new(),
            intent_path,
            preparation_authority_path,
            _directory_lock: directory_lock,
            _lifecycle_lock: lifecycle_lock,
        })
    }

    pub fn namespace(&self) -> &'a NamespaceLease {
        self.namespace
    }

    pub const fn terminal_id(&self) -> Uuid {
        self.terminal_id
    }

    pub const fn session_identity(&self) -> ObjectIdentity {
        self.session_identity
    }

    pub fn require_no_pending_intent(&self) -> Result<()> {
        self.verify_session_directory()?;
        require_absent_at(self.namespace, &self.intent_path)?;
        self.verify_session_directory()
    }

    pub fn read_optional_with_identity<T>(&self, path: &Path) -> Result<Option<(T, FileIdentity)>>
    where
        T: DeserializeOwned,
    {
        self.require_managed_path(path)?;
        if self.namespace.file_is_missing(path)? {
            return Ok(None);
        }
        self.namespace.read_json_with_identity(path).map(Some)
    }

    pub fn read_intent_optional_with_identity<T>(&self) -> Result<Option<(T, FileIdentity)>>
    where
        T: DeserializeOwned,
    {
        self.verify_session_directory()?;
        if self.namespace.file_is_missing(&self.intent_path)? {
            return Ok(None);
        }
        self.namespace
            .read_json_with_identity(&self.intent_path)
            .map(Some)
    }

    pub fn verify_session_directory(&self) -> Result<()> {
        self.namespace.verify()?;
        self.session_directory.verify_path()?;
        ensure!(
            self.session_directory.object_identity()? == self.session_identity,
            SESSION_DIRECTORY_MISMATCH
        );
        Ok(())
    }

    pub fn require_managed_path(&self, path: &Path) -> Result<()> {
        ensure!(
            path.parent() == Some(self.session_directory.path())
                && path != self.intent_path
                && path != self.preparation_authority_path
                && path.file_name() != Some(SESSION_LIFECYCLE_LOCK_FILE_NAME.as_ref()),
            PARTICIPANT_PATH_INVALID
        );
        self.verify_session_directory()
    }

    pub fn require_preparation_authority_path(&self, path: &Path) -> Result<()> {
        ensure!(
            path == self.preparation_authority_path,
            PARTICIPANT_PATH_INVALID
        );
        self.verify_session_directory()
    }
}

fn require_absent_at(namespace: &NamespaceLease, intent_path: &Path) -> Result<()> {
    ensure!(namespace.file_is_missing(intent_path)?, PENDING_INTENT);
    namespace.verify()?;
    ensure!(namespace.file_is_missing(intent_path)?, PENDING_INTENT);
    Ok(())
}
