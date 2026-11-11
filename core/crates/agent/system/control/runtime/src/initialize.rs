use crate::{
    ControlRuntime, FreshInitialization, JournalError,
    anchor::AnchorRecord,
    constants::*,
    storage::{self, Locations},
};
use std::sync::atomic::AtomicBool;
use yolo_agent_control_journal::{Journal, JournalScope};
use yolo_paths::{
    Paths, directory::PrivateDirectory, durable_mutation::SessionDurableMutationGuard,
};
use yolo_terminal_model::RuntimeRecord;

impl ControlRuntime {
    pub fn initialize_fresh(proof: FreshInitialization) -> Result<Self, JournalError> {
        proof.verify().map_err(|_| JournalError::Unavailable)?;
        let current = &proof.runtime;
        let scope = JournalScope::new(current.terminal_id, current.runtime_epoch)?;
        let locations = Locations::new(
            &proof.paths.session_dir(current.terminal_id),
            current.runtime_epoch,
        );
        let guard = SessionDurableMutationGuard::acquire_participant(
            &proof.paths,
            &proof.namespace,
            current.terminal_id,
        )
        .map_err(|_| JournalError::Unavailable)?;
        guard
            .require_no_pending_intent()
            .map_err(|_| JournalError::Unavailable)?;
        proof.verify().map_err(|_| JournalError::Unavailable)?;
        if !proof
            .namespace
            .file_is_missing(&locations.anchor)
            .map_err(|_| JournalError::Unavailable)?
        {
            return Err(JournalError::AlreadyExists);
        }
        let directory = proof
            .namespace
            .ensure_directory(&locations.journal)
            .map_err(|_| JournalError::Unavailable)?;
        let journal = Journal::create_new(directory, scope)?;
        let record = AnchorRecord {
            schema: SCHEMA,
            runtime: current.clone(),
            journal: journal.anchor(),
        };
        record
            .validate(current)
            .map_err(|_| JournalError::ScopeMismatch)?;
        let identity = guard
            .write_json_expected_with_fence_identity(&locations.anchor, None, &record, |_| {
                proof.verify()?;
                journal.verify()?;
                Ok(())
            })
            .map_err(|_| JournalError::OutcomeUnknown)?;
        drop(guard);
        let instance = Self {
            namespace: proof.namespace,
            anchor_path: locations.anchor,
            record,
            identity,
            journal,
            poisoned: AtomicBool::new(false),
        };
        instance.verify(&proof.runtime)?;
        Ok(instance)
    }

    /// Existing runtimes never initialize, including when every control artifact is absent.
    pub fn open_existing(
        session: PrivateDirectory,
        paths: &Paths,
        current: &RuntimeRecord,
    ) -> Result<Self, JournalError> {
        let scope = JournalScope::new(current.terminal_id, current.runtime_epoch)?;
        let namespace = storage::retain(session, paths, current.terminal_id)?;
        let locations = Locations::new(
            &paths.session_dir(current.terminal_id),
            current.runtime_epoch,
        );
        let (record, identity): (AnchorRecord, _) = namespace
            .read_json_with_identity(&locations.anchor)
            .map_err(|_| JournalError::Unavailable)?;
        record
            .validate(current)
            .map_err(|_| JournalError::ScopeMismatch)?;
        let directory = namespace
            .directory(&locations.journal)
            .map_err(|_| JournalError::Unavailable)?;
        let journal = Journal::open_existing(directory, scope, record.journal)?;
        let instance = Self {
            namespace,
            anchor_path: locations.anchor,
            record,
            identity,
            journal,
            poisoned: AtomicBool::new(false),
        };
        instance.verify(current)?;
        Ok(instance)
    }
}
