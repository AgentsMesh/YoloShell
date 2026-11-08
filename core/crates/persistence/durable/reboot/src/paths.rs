use crate::agent::claude::generation::ClaudeGenerationStore;
use crate::paths::Paths;
use anyhow::Result;
use std::path::PathBuf;
use uuid::Uuid;

pub(super) struct ParticipantPaths {
    pub(super) startup_checkpoint: PathBuf,
    pub(super) claude_custody: PathBuf,
    pub(super) claude_generation: PathBuf,
    pub(super) pty_custody: PathBuf,
    pub(super) runtime: PathBuf,
}

impl ParticipantPaths {
    pub(super) fn new(paths: &Paths, terminal_id: Uuid) -> Result<Self> {
        let session = paths.session_dir(terminal_id);
        let generation =
            ClaudeGenerationStore::from_binding_path(&paths.agent_binding(terminal_id))?;
        let participant_paths = Self {
            startup_checkpoint: paths.worker_startup_checkpoint(terminal_id),
            claude_custody: session.join(crate::claude_custody::LEDGER_FILE),
            claude_generation: generation.path().to_owned(),
            pty_custody: paths.pty_custody(terminal_id),
            runtime: paths.runtime_record(terminal_id),
        };
        participant_paths.validate(&session)?;
        Ok(participant_paths)
    }

    fn validate(&self, session: &std::path::Path) -> Result<()> {
        for path in [
            &self.startup_checkpoint,
            &self.claude_custody,
            &self.claude_generation,
            &self.pty_custody,
            &self.runtime,
        ] {
            anyhow::ensure!(
                path.parent() == Some(session),
                super::constants::PARTICIPANT_PATH_INVALID
            );
        }
        Ok(())
    }
}
