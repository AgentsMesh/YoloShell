use crate::{
    SSH_PROFILE_REVISION_STEP, SshProfileChange, SshProfileState, SshProfileStore,
    checkpoint::{self, Checkpoint},
    constants::*,
    record::Record,
};
use anyhow::{Result, ensure};

#[derive(Debug)]
pub struct RevisionConflict;
impl std::fmt::Display for RevisionConflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(REVISION_CONFLICT)
    }
}
impl std::error::Error for RevisionConflict {}
#[derive(Debug)]
pub struct RevisionExhausted;
impl std::fmt::Display for RevisionExhausted {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(REVISION_EXHAUSTED)
    }
}
impl std::error::Error for RevisionExhausted {}

impl SshProfileStore {
    pub fn apply(&mut self, change: SshProfileChange) -> Result<SshProfileState> {
        change.validate()?;
        let current = self.current()?;
        if current.last_change == change.request_id {
            ensure!(
                change
                    .expected_revision
                    .checked_add(SSH_PROFILE_REVISION_STEP)
                    == Some(current.revision)
                    && change.configuration == current.configuration,
                RevisionConflict
            );
            return Ok(current);
        }
        ensure!(
            current.revision == change.expected_revision,
            RevisionConflict
        );
        let state = SshProfileState {
            hub: current.hub,
            revision: current
                .revision
                .checked_add(SSH_PROFILE_REVISION_STEP)
                .ok_or(RevisionExhausted)?,
            last_change: change.request_id,
            configuration: change.configuration,
        };
        let record = Record {
            schema_version: RECORD_SCHEMA,
            directory: self.record.directory,
            state: state.clone(),
        };
        record.validate(self.record.directory, self.record.state.hub)?;
        let result = (|| {
            self.verify_namespace()?;
            let written = self.namespace.write_json_expected_with_commit_fence(
                &self.path,
                Some(self.file),
                &record,
                || {
                    checkpoint::reach(Checkpoint::BeforeCommit)?;
                    self.read_current().map(|_| ())
                },
            )?;
            checkpoint::reach(Checkpoint::AfterCommit)?;
            Ok(written)
        })();
        self.file = self.retain_failure(result)?;
        self.record = record;
        self.current()
    }
}
