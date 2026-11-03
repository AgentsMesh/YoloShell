use crate::{
    Journal, JournalAnchor, JournalError, JournalScope,
    constants::*,
    record::{Header, Record},
};
use std::{path::PathBuf, sync::atomic::AtomicBool};
use uuid::Uuid;
use yolo_paths::{
    coordination::CoordinationLock,
    directory::PrivateDirectory,
    identity::{FileIdentity, ObjectIdentity},
    namespace_lease::NamespaceLease,
};

impl Journal {
    /// Only the trusted initializer may call this, before publishing runtime Ready.
    /// Persist anchor() outside this directory. Never use creation as a retry fallback.
    pub fn create_new(
        directory: PrivateDirectory,
        scope: JournalScope,
    ) -> Result<Self, JournalError> {
        let context = Context::retain(directory)?;
        let absent = context
            .namespace
            .file_is_missing(&context.header_path)
            .and_then(|header_missing| {
                context
                    .namespace
                    .file_is_missing(&context.record_path)
                    .map(|record_missing| header_missing && record_missing)
            })
            .map_err(|_| JournalError::Unavailable)?;
        if !absent {
            return Err(JournalError::AlreadyExists);
        }
        let header = Header::new(scope, context.directory);
        let header_identity = context
            .namespace
            .write_json_expected_with_commit_fence(&context.header_path, None, &header, || {
                context.lock.verify()
            })
            .map_err(|_| JournalError::OutcomeUnknown)?;
        let record = Record::empty(header.anchor(header_identity));
        let record_identity = context
            .namespace
            .write_json_expected_with_commit_fence(&context.record_path, None, &record, || {
                context.lock.verify()?;
                let (current, identity): (Header, _) = context
                    .namespace
                    .read_json_with_identity(&context.header_path)?;
                anyhow::ensure!(
                    current == header && identity == header_identity,
                    REPLACED_HEADER
                );
                Ok(())
            })
            .map_err(|_| JournalError::OutcomeUnknown)?;
        context.finish(header, header_identity, record, record_identity)
    }

    /// Missing history, changed immutable headers and malformed records are errors.
    /// The header anchor does not detect rollback of a valid mutable receipt record.
    pub fn open_existing(
        directory: PrivateDirectory,
        scope: JournalScope,
        expected: JournalAnchor,
    ) -> Result<Self, JournalError> {
        if expected.scope != scope {
            return Err(JournalError::ScopeMismatch);
        }
        let context = Context::retain(directory)?;
        if expected.directory != context.directory {
            return Err(JournalError::ScopeMismatch);
        }
        let (header, header_identity): (Header, _) = context
            .namespace
            .read_json_with_identity(&context.header_path)
            .map_err(|_| JournalError::Unavailable)?;
        if header_identity != expected.header_identity {
            return Err(JournalError::ScopeMismatch);
        }
        header
            .validate(expected)
            .map_err(|_| JournalError::ScopeMismatch)?;
        let (record, record_identity): (Record, _) = context
            .namespace
            .read_json_with_identity(&context.record_path)
            .map_err(|_| JournalError::Unavailable)?;
        record
            .validate(expected)
            .map_err(|_| JournalError::Unavailable)?;
        context.finish(header, header_identity, record, record_identity)
    }
}

struct Context {
    directory: ObjectIdentity,
    header_path: PathBuf,
    record_path: PathBuf,
    namespace: NamespaceLease,
    lock: CoordinationLock,
}
impl Context {
    fn retain(directory: PrivateDirectory) -> Result<Self, JournalError> {
        let identity = directory
            .object_identity()
            .map_err(|_| JournalError::Unavailable)?;
        let root = directory.path().to_path_buf();
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])
            .map_err(|_| JournalError::Unavailable)?;
        let lock = CoordinationLock::try_acquire(&root.join(LOCK_FILE), LOCK_BYTES)
            .map_err(|_| JournalError::Unavailable)?
            .ok_or(JournalError::Busy)?;
        namespace.verify().map_err(|_| JournalError::Unavailable)?;
        Ok(Self {
            directory: identity,
            header_path: root.join(HEADER_FILE),
            record_path: root.join(JOURNAL_FILE),
            namespace,
            lock,
        })
    }
    fn finish(
        self,
        header: Header,
        header_identity: FileIdentity,
        record: Record,
        record_identity: FileIdentity,
    ) -> Result<Journal, JournalError> {
        let journal = Journal {
            header_path: self.header_path,
            record_path: self.record_path,
            namespace: self.namespace,
            lock: self.lock,
            header,
            header_identity,
            record,
            record_identity,
            owner: Uuid::new_v4(),
            poisoned: AtomicBool::new(false),
        };
        journal.verify()?;
        Ok(journal)
    }
}
