use crate::{SetupRecord, constants::*, model::BoundRecord};
use anyhow::{Context, Result, ensure};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, identity::FileIdentity,
    namespace_lease::NamespaceLease,
};

pub enum OpenedSetup {
    Fresh(SetupOperation),
    Existing(SetupOperation),
}
pub struct SetupOperation {
    path: PathBuf,
    namespace: NamespaceLease,
    lock: CoordinationLock,
    record: BoundRecord,
    file: FileIdentity,
    lost: AtomicBool,
}

impl SetupOperation {
    pub fn open(parent: PrivateDirectory, proposed: SetupRecord) -> Result<OpenedSetup> {
        let parent = crate::parent::retain(parent)?;
        parent.verify_path()?;
        proposed.validate()?;
        let id = proposed.request.request.fields().request_id;
        let lock = CoordinationLock::try_acquire(
            &parent.path().join(format!("{id}{LOCK_SUFFIX}")),
            LOCK_BYTES,
        )?
        .context(BUSY)?;
        parent.verify_path()?;
        let directory_path = parent.path().join(id.to_string());
        let existing = std::fs::symlink_metadata(&directory_path)
            .map(|_| true)
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(false)
                } else {
                    Err(error)
                }
            })?;
        ensure!(
            existing || matches!(proposed.phase, crate::SetupPhase::Prepared {}),
            CONFLICT
        );
        let directory = if existing {
            parent.open_descendant(&directory_path)?
        } else {
            parent.ensure_descendant(&directory_path)?
        };
        if !existing {
            directory.sync()?;
            parent.sync()?;
        }
        let path = directory.path().join(RECORD_FILE);
        let bound = BoundRecord {
            parent: parent.durable_identity()?,
            directory: directory.durable_identity()?,
            record: proposed,
        };
        let namespace =
            NamespaceLease::from_directories(vec![(PARENT, parent), (ENTRY, directory)])?;
        let (record, file) = if existing {
            namespace.read_json_with_identity::<BoundRecord>(&path)?
        } else {
            let file =
                namespace
                    .write_json_expected_with_commit_fence(&path, None, &bound, || lock.verify())?;
            (bound.clone(), file)
        };
        record.record.validate()?;
        ensure!(
            record.parent == bound.parent
                && record.directory == bound.directory
                && record.record.owner == bound.record.owner
                && record.record.request == bound.record.request
                && record.record.registration == bound.record.registration,
            CONFLICT
        );
        let operation = Self {
            path,
            namespace,
            lock,
            record,
            file,
            lost: AtomicBool::new(false),
        };
        operation.verify()?;
        Ok(if existing {
            OpenedSetup::Existing(operation)
        } else {
            OpenedSetup::Fresh(operation)
        })
    }
    pub fn inspect(parent: &PrivateDirectory, id: Uuid) -> Result<Option<SetupRecord>> {
        Ok(crate::SetupObservation::observe(parent, id)?.map(|observed| observed.record().clone()))
    }
    pub fn record(&self) -> &SetupRecord {
        &self.record.record
    }
    pub fn verify(&self) -> Result<()> {
        let result = (|| {
            ensure!(!self.lost.load(Ordering::Acquire), LOST);
            self.lock.verify()?;
            self.namespace.verify()?;
            ensure!(
                self.namespace
                    .read_json_with_identity::<BoundRecord>(&self.path)?
                    == (self.record.clone(), self.file),
                CONFLICT
            );
            self.namespace.verify()
        })();
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        result
    }
    pub fn transition(&mut self, phase: crate::SetupPhase) -> Result<()> {
        self.verify()?;
        ensure!(
            matches!(
                (&self.record.record.phase, &phase),
                (
                    crate::SetupPhase::Prepared {},
                    crate::SetupPhase::Provisioning { .. }
                ) | (
                    crate::SetupPhase::Provisioning { .. },
                    crate::SetupPhase::Routing { .. }
                ) | (
                    crate::SetupPhase::Routing { .. },
                    crate::SetupPhase::Complete { .. }
                )
            ),
            CONFLICT
        );
        match (&self.record.record.phase, &phase) {
            (
                crate::SetupPhase::Provisioning {
                    broker_directory: old,
                },
                crate::SetupPhase::Routing {
                    broker_directory: next,
                    ..
                },
            ) => ensure!(old == next, CONFLICT),
            (
                crate::SetupPhase::Routing {
                    broker_directory: old,
                    change: old_change,
                    provision: old_provision,
                },
                crate::SetupPhase::Complete {
                    broker_directory: next,
                    change,
                    provision,
                },
            ) => {
                ensure!(
                    old == next && old_change == change && old_provision == provision,
                    CONFLICT
                );
            }
            _ => {}
        }
        let mut next = self.record.clone();
        next.record.phase = phase;
        next.record.validate()?;
        let result = self.namespace.write_json_expected_with_commit_fence(
            &self.path,
            Some(self.file),
            &next,
            || self.verify(),
        );
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        self.file = result?;
        self.record = next;
        self.verify()
    }
}
