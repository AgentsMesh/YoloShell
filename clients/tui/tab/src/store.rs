use crate::{
    LayoutCheckpoint, LocalTabView, TabCatalog, TabDocument, constants::*, migration,
};
use anyhow::{Context, Result, anyhow, ensure};
use serde_json::{Error as JsonError, Value};
use std::path::{Path, PathBuf};
use yolo_hub_identity::HubId;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, identity::FileIdentity,
    namespace_lease::NamespaceLease,
};

pub struct TabStore {
    namespace: NamespaceLease,
    lock: CoordinationLock,
    path: PathBuf,
    identity: FileIdentity,
    document: TabDocument,
    lost: bool,
}
impl TabStore {
    pub fn open(directory: &Path, entry: HubId) -> Result<Self> {
        Self::try_open(directory, entry)?.context(IN_USE)
    }
    /// Lock contention is a typed result so client allocation can choose another directory.
    pub fn try_open(directory: &Path, entry: HubId) -> Result<Option<Self>> {
        let directory = yolo_paths::platform::platform_path(directory);
        let directory = PrivateDirectory::ensure(&directory)?;
        Self::try_open_retained(directory, entry)
    }
    pub(crate) fn try_open_retained(
        directory: PrivateDirectory,
        entry: HubId,
    ) -> Result<Option<Self>> {
        directory.verify_path()?;
        let path = directory.path().join(TAB_FILE);
        let Some(lock) =
            CoordinationLock::try_acquire(&directory.path().join(LOCK_FILE), LOCK_BYTES)?
        else {
            return Ok(None);
        };
        directory.verify_path()?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let (mut document, mut identity) = if namespace.file_is_missing(&path)? {
            let document = TabDocument {
                schema_version: TAB_SCHEMA_VERSION,
                entry_hub: entry,
                layout_address: yolo_presentation_protocol::LayoutAddress {
                    hub_id: entry,
                    layout_id: uuid::Uuid::new_v4(),
                },
                shared: None,
                revision: FIRST_TAB_REVISION,
                tabs: TabCatalog::new(entry, LocalTabView::default()),
            };
            let identity =
                namespace.write_json_expected_with_commit_fence(&path, None, &document, || {
                    lock.verify()
                })?;
            (document, identity)
        } else {
            let (value, identity) = namespace.read_json_with_identity::<Value>(&path)?;
            match migration::schema_version(&value) {
                Some(version) if version == TAB_SCHEMA_VERSION => {
                    (serde_json::from_value::<TabDocument>(value)?, identity)
                }
                Some(version) if version == migration::LEGACY_SCHEMA_VERSION => {
                    let document = migration::migrate_v5(value, entry)?;
                    document.validate(entry)?;
                    let identity = namespace.write_json_expected_with_commit_fence(
                        &path,
                        Some(identity),
                        &document,
                        || lock.verify(),
                    )?;
                    (document, identity)
                }
                _ => return Err(anyhow!(SCHEMA_ERROR)),
            }
        };
        document.validate(entry)?;
        Ok(Some(Self {
            namespace,
            lock,
            path,
            identity,
            document,
            lost: false,
        }))
    }
    pub fn document(&self) -> &TabDocument {
        &self.document
    }
    pub fn select_layout(&mut self, id: uuid::Uuid) -> Result<()> {
        let mut next = self.document.clone();
        next.layout_address.layout_id = id;
        next.shared = None;
        next.revision = next
            .revision
            .checked_add(FIRST_TAB_REVISION)
            .context(REVISION_ERROR)?;
        next.validate(next.entry_hub)?;
        self.directory()?;
        let result = self.persist(next);
        if result.is_err() {
            self.lost = true;
        }
        result
    }
    /// Retain the same namespace for private companion state, without reopening a replacement.
    pub fn directory(&self) -> Result<PrivateDirectory> {
        self.verify_current()?;
        self.namespace.directory(self.path.parent().context(LOST)?)
    }
    /// Verify ownership and the persisted document without writing it.
    ///
    /// The TUI uses this on its idle path so the dirty-gate can avoid a
    /// durable write while retaining the external replacement conflict fence.
    pub fn verify_current(&self) -> Result<()> {
        ensure!(!self.lost, LOST);
        self.lock.verify()?;
        self.namespace.verify()?;
        self.namespace
            .file(&self.path)?
            .verify_exact_identity(self.identity, yolo_paths::constants::MAX_DURABLE_JSON_BYTES)
    }
    pub fn save(&mut self, tab: &LocalTabView) -> Result<bool> {
        let mut tabs = self.document.tabs.clone();
        tabs.update_active(tab.clone());
        self.save_catalog(&tabs)
    }
    pub fn save_catalog(&mut self, tabs: &TabCatalog) -> Result<bool> {
        self.save_presentation(tabs, self.document.shared.clone())
    }
    pub fn save_presentation(
        &mut self,
        tabs: &TabCatalog,
        shared: Option<LayoutCheckpoint>,
    ) -> Result<bool> {
        ensure!(!self.lost, LOST);
        let result = self.save_current(tabs, shared);
        if result.is_err() {
            self.lost = true;
        }
        result
    }
    fn save_current(
        &mut self,
        tabs: &TabCatalog,
        shared: Option<LayoutCheckpoint>,
    ) -> Result<bool> {
        self.namespace.verify()?;
        self.lock.verify()?;
        // Content identity is checked even on an unchanged view: external replacement
        // is a conflict, not permission to overwrite the replacement on the next edit.
        self.namespace
            .file(&self.path)?
            .verify_exact_identity(self.identity, yolo_paths::constants::MAX_DURABLE_JSON_BYTES)?;
        if &self.document.tabs == tabs && self.document.shared == shared {
            return Ok(false);
        }
        let mut next = self.document.clone();
        next.revision = next
            .revision
            .checked_add(FIRST_TAB_REVISION)
            .context(REVISION_ERROR)?;
        next.tabs = tabs.clone();
        next.shared = shared;
        next.validate(next.entry_hub)?;
        self.persist(next)?;
        Ok(true)
    }
    fn persist(&mut self, next: TabDocument) -> Result<()> {
        let identity = self.namespace.write_json_expected_with_commit_fence(
            &self.path,
            Some(self.identity),
            &next,
            || self.lock.verify(),
        )?;
        self.identity = identity;
        self.document = next;
        Ok(())
    }
}

pub(crate) fn is_document_decode_error(error: &anyhow::Error) -> bool {
    error
        .chain()
        .any(|cause| cause.downcast_ref::<JsonError>().is_some())
}
