impl StoppedOutputContext {
    fn verify(
        &self,
        isolated: &process::LiveProcessIdentity,
        require_completion: bool,
    ) -> Result<()> {
        let published = self
            .read_identity(StoppedOutputIdentityRole::Isolated)?
            .context(STOPPED_OUTPUT_CONTEXT_DETAIL)?;
        anyhow::ensure!(
            published.live == *isolated,
            STOPPED_OUTPUT_IDENTITY_CHANGED_DETAIL
        );
        anyhow::ensure!(
            self.read_identity(StoppedOutputIdentityRole::Root)?
                .is_some(),
            STOPPED_OUTPUT_CONTEXT_DETAIL
        );
        self.verify_completion(require_completion)
    }

    fn verify_completion(&self, required: bool) -> Result<()> {
        let completion = self.directory.join(STOPPED_OUTPUT_COMPLETED_FILE);
        if !required && !completion.try_exists()? {
            return Ok(());
        }
        let completed: StoppedOutputCompletionDocument = read_json(&completion)?;
        anyhow::ensure!(
            completed.nonce == self.nonce && completed.case == self.case.name(),
            STOPPED_OUTPUT_CONTEXT_INVALID_DETAIL
        );
        Ok(())
    }
}

#[test]
fn failed_stopped_output_completion_is_optional_but_validated_when_present() {
    let directory = tempfile::tempdir().unwrap();
    let context = StoppedOutputContext {
        directory: directory.path().to_owned(),
        nonce: Uuid::new_v4().to_string(),
        case: STOPPED_OUTPUT_STARTUP_CTRL_S_CASE,
    };

    context.verify_completion(false).unwrap();
    assert!(context.verify_completion(true).is_err());
    context.complete().unwrap();
    context.verify_completion(false).unwrap();
    let mut mismatched = context.clone();
    mismatched.nonce = Uuid::new_v4().to_string();
    assert!(mismatched.verify_completion(false).is_err());
}
