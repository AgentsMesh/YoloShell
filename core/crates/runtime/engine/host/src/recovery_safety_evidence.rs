use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ExitRecordState {
    Missing,
    Matching(ExitReason),
    Stale,
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TerminationTombstoneState {
    Missing,
    Matching,
    Invalid,
}

pub(super) const EXIT_RECORD_NAMESPACE_UNTRUSTWORTHY_DETAIL: &str = "exit record namespace is untrustworthy";
pub(super) const EXIT_RECORD_UNREADABLE_DETAIL: &str = "exit record is unreadable";
pub(super) const TOMBSTONE_IDENTITY_INVALID_DETAIL: &str = "termination tombstone identity is invalid";
pub(super) const TOMBSTONE_NAMESPACE_UNTRUSTWORTHY_DETAIL: &str =
    "termination tombstone namespace is untrustworthy";
pub(super) const TOMBSTONE_UNREADABLE_DETAIL: &str = "termination tombstone is unreadable";

impl Host {
    pub(super) fn automatic_resume_block(
        &self,
        manifest: &TerminalManifest,
        expected_runtime: &RuntimeRecord,
    ) -> AutomaticResumeBlock {
        if self.stable_user_termination_matches(manifest, expected_runtime) {
            AutomaticResumeBlock::UserTermination
        } else {
            AutomaticResumeBlock::RecoveryEvidenceChanged
        }
    }

    pub(super) fn stable_user_termination_matches(
        &self,
        manifest: &TerminalManifest,
        expected_runtime: &RuntimeRecord,
    ) -> bool {
        manifest.terminal_id == expected_runtime.terminal_id
            && self.stable_termination_tombstone_matches(expected_runtime)
    }

    pub(super) fn stable_termination_tombstone_matches(&self, expected_runtime: &RuntimeRecord) -> bool {
        let runtime_path = self.paths.runtime_record(expected_runtime.terminal_id);
        let Ok((runtime, runtime_identity)) = self
            .namespace
            .read_json_with_identity::<RuntimeRecord>(&runtime_path)
        else {
            return false;
        };
        if runtime != *expected_runtime {
            return false;
        }
        let tombstone_path = self.paths.terminate_tombstone(expected_runtime.terminal_id);
        let Ok((tombstone, tombstone_identity)) = self
            .namespace
            .read_json_with_identity::<crate::model::TerminationTombstone>(&tombstone_path)
        else {
            return false;
        };
        if tombstone
            .validate_for(runtime.terminal_id, runtime.runtime_epoch)
            .is_err()
        {
            return false;
        }
        let tombstone_stable = self
            .namespace
            .read_json_with_identity::<crate::model::TerminationTombstone>(&tombstone_path)
            .is_ok_and(|current| current == (tombstone, tombstone_identity));
        let runtime_stable = self
            .namespace
            .read_json_with_identity::<RuntimeRecord>(&runtime_path)
            .is_ok_and(|current| current == (runtime, runtime_identity));
        tombstone_stable && runtime_stable
    }

    pub(super) fn exit_allows_restore(&self, runtime: &RuntimeRecord) -> bool {
        matches!(
            self.exit_record_state(runtime),
            ExitRecordState::Missing
                | ExitRecordState::Stale
                | ExitRecordState::Matching(ExitReason::WorkerCrash)
        )
    }

    pub(super) fn exit_record_state(&self, runtime: &RuntimeRecord) -> ExitRecordState {
        let path = self.paths.exit_record(runtime.terminal_id);
        match self.namespace.file_is_missing(&path) {
            Ok(true) => ExitRecordState::Missing,
            Err(error) => {
                tracing::error!(terminal_id = %runtime.terminal_id, %error,
                    detail = EXIT_RECORD_NAMESPACE_UNTRUSTWORTHY_DETAIL);
                ExitRecordState::Invalid
            }
            Ok(false) => match self.read_namespace_json::<ExitRecord>(&path) {
                Ok(exit) if exit.runtime_epoch != runtime.runtime_epoch => ExitRecordState::Stale,
                Ok(exit) => ExitRecordState::Matching(exit.reason),
                Err(error) => {
                    tracing::error!(terminal_id = %runtime.terminal_id, %error,
                        detail = EXIT_RECORD_UNREADABLE_DETAIL);
                    ExitRecordState::Invalid
                }
            },
        }
    }

    pub(super) fn termination_tombstone_state(&self, runtime: &RuntimeRecord) -> TerminationTombstoneState {
        let path = self.paths.terminate_tombstone(runtime.terminal_id);
        match self.namespace.file_is_missing(&path) {
            Ok(true) => TerminationTombstoneState::Missing,
            Err(error) => {
                tracing::error!(terminal_id = %runtime.terminal_id, %error,
                    detail = TOMBSTONE_NAMESPACE_UNTRUSTWORTHY_DETAIL);
                TerminationTombstoneState::Invalid
            }
            Ok(false) => self.read_tombstone_state(runtime, &path),
        }
    }

    pub(super) fn read_tombstone_state(
        &self,
        runtime: &RuntimeRecord,
        path: &Path,
    ) -> TerminationTombstoneState {
        match self.read_namespace_json::<crate::model::TerminationTombstone>(path) {
            Ok(tombstone) => {
                match tombstone.validate_for(runtime.terminal_id, runtime.runtime_epoch) {
                    Ok(()) => TerminationTombstoneState::Matching,
                    Err(error) => {
                        tracing::error!(terminal_id = %runtime.terminal_id, %error,
                        detail = TOMBSTONE_IDENTITY_INVALID_DETAIL);
                        TerminationTombstoneState::Invalid
                    }
                }
            }
            Err(error) => {
                tracing::error!(terminal_id = %runtime.terminal_id, %error,
                    detail = TOMBSTONE_UNREADABLE_DETAIL);
                TerminationTombstoneState::Invalid
            }
        }
    }
}
