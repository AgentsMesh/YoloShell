use crate::{ConfigurationStore, constants::*};
use anyhow::{Result, ensure};
use yolo_hub_management_protocol::{
    CONFIGURATION_REVISION_STEP, CONFIGURATION_SCHEMA, ConfigurationChange, ConfigurationState,
};

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

impl ConfigurationStore {
    /// An exact replay of the latest committed change returns its resulting state.
    /// Callers must not repeat runtime application when this revision is already applied.
    pub fn check(&self, change: &ConfigurationChange) -> Result<ConfigurationState> {
        change.validate(self.identity)?;
        let current = self.current()?;
        if current.last_change == change.request_id {
            ensure!(replays(&current, change), RevisionConflict);
        } else {
            ensure!(
                current.revision == change.expected_revision,
                RevisionConflict
            );
            current
                .revision
                .checked_add(CONFIGURATION_REVISION_STEP)
                .ok_or(RevisionExhausted)?;
        }
        Ok(current)
    }

    pub fn apply(&mut self, change: ConfigurationChange) -> Result<ConfigurationState> {
        let current = self.check(&change)?;
        if replays(&current, &change) {
            return Ok(current);
        }
        let state = self.planned(&change)?;
        self.write(state.clone())?;
        Ok(state)
    }

    /// The exact state a successful apply will persist, without writing it.
    pub fn planned(&self, change: &ConfigurationChange) -> Result<ConfigurationState> {
        let current = self.check(change)?;
        if replays(&current, change) {
            return Ok(current);
        }
        Ok(ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: self.identity,
            revision: current
                .revision
                .checked_add(CONFIGURATION_REVISION_STEP)
                .ok_or(RevisionExhausted)?,
            last_change: change.request_id,
            configuration: change.configuration.clone(),
        })
    }
}

fn replays(current: &ConfigurationState, change: &ConfigurationChange) -> bool {
    current.last_change == change.request_id
        && change
            .expected_revision
            .checked_add(CONFIGURATION_REVISION_STEP)
            == Some(current.revision)
        && current.configuration == change.configuration
}
