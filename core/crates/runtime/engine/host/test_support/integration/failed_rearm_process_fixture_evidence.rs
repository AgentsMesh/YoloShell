#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedEvidence {
    controller_generation: u64,
    terminal_id: Uuid,
    transaction: ResumeTransaction,
    runtime: RuntimeRecord,
    checkpoint: StartupCheckpointEvidence,
}

impl PreparedEvidence {
    pub fn encode(&self) -> Result<String> {
        serde_json::to_string(self).context(FIXTURE_ENCODE_CONTEXT)
    }

    pub fn decode(value: &str) -> Result<Self> {
        serde_json::from_str(value).context(FIXTURE_DECODE_CONTEXT)
    }

    fn validate_home(&self, home: &Path) -> Result<()> {
        ensure!(
            self.transaction.terminal_id == self.terminal_id
                && self.runtime.terminal_id == self.terminal_id
                && self.checkpoint.terminal_id == self.terminal_id
                && self.transaction.launch.cwd == home
                && self.runtime.cwd == home,
            FIXTURE_HOME_MISMATCH
        );
        Ok(())
    }

    fn socket_path(&self) -> &Path {
        &self.transaction.launch.socket_path
    }
}
