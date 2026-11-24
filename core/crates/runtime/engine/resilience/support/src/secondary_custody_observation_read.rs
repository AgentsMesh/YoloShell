use super::*;
use crate::paths::private_file::PrivateFileEntry;
use std::sync::Arc;

const OBSERVATION_LOCK_TIMEOUT: std::time::Duration =
    crate::session_timing::PTY_CAPABILITY_SETTLEMENT_TIMEOUT;
const OBSERVATION_LOCK_RETRY: std::time::Duration =
    crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;

impl ActiveCustodyObservation {
    pub fn read(paths: &Paths, terminal_id: Uuid) -> Result<Self> {
        Self::read_observing_session_lock(paths, terminal_id, || Ok(()))
    }

    pub fn read_observing_session_lock(
        paths: &Paths,
        terminal_id: Uuid,
        after_lock: impl FnOnce() -> Result<()>,
    ) -> Result<Self> {
        let session_dir = paths.session_dir(terminal_id);
        let namespace = NamespaceLease::capture(&[(OBSERVATION_NAMESPACE_LABEL, &session_dir)])
            .context(READ_CONTEXT)?;
        let entry = Arc::new(PrivateFileEntry::open(&paths.pty_custody(terminal_id))?);
        // Session record writers acquire the directory before locking their target file.
        let _session_lock =
            entry.lock_parent_exclusive_wait(OBSERVATION_LOCK_TIMEOUT, OBSERVATION_LOCK_RETRY)?;
        after_lock()?;
        let (record, record_observation): (CustodyDurableRecord, _) =
            RetainedJsonFileObservation::read_entry_guarded_with_shared_lock(
                entry,
                || namespace.verify(),
                OBSERVATION_LOCK_TIMEOUT,
                OBSERVATION_LOCK_RETRY,
            )
            .context(READ_CONTEXT)?;
        ensure!(record.is_valid(), INVALID_RECORD_DETAIL);
        ensure!(
            record.runtime.terminal_id == terminal_id,
            SESSION_MISMATCH_DETAIL
        );
        let lease = record.armed_lease().context(ACTIVE_LEASE_REQUIRED)?;
        lease.guardian.context(GUARDIAN_REQUIRED)?;
        let (runtime, runtime_identity): (crate::model::RuntimeRecord, FileIdentity) = namespace
            .read_json_with_identity(&paths.runtime_record(terminal_id))
            .context(READ_CONTEXT)?;
        ensure!(
            record.runtime.matches_record(&runtime)
                && lease.controller_generation == runtime.controller_generation,
            RUNTIME_MISMATCH_DETAIL
        );
        let record_identity = record_observation.identity();
        let probe = crate::custody_guardian::test_probe::read_exact_observation(
            &namespace,
            paths,
            &record,
            record_identity,
        )?;
        crate::custody_guardian::test_probe::confirm_durable_unchanged(
            &namespace,
            paths,
            &record,
            record_identity,
        )?;
        Ok(Self {
            record,
            record_identity,
            runtime_identity,
            probe,
            _record_observation: record_observation,
        })
    }
}
