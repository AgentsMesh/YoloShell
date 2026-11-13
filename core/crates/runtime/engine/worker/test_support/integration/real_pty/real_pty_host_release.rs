pub(super) type GatedHostRelease = FixtureHostRelease;

impl GatedHostRelease {
    fn prepare(
        fixture: &Fixture,
        capability: &FixturePtyCapability,
        root: process::ProcessIdentity,
    ) -> (
        process::PtySessionProof,
        crate::worker::pty_output_progress::ReaderBackedPtyOutput,
        Self,
    ) {
        let mut proof = capability.proof(fixture);
        let record = lock_recover(&fixture.state.record);
        proof
            .bind_runtime(record.terminal_id, record.runtime_epoch)
            .unwrap();
        drop(record);
        let authority = proof.bind_master_alias_release(root).unwrap();
        let master = lock_recover(&fixture.state.master);
        let output =
            crate::worker::pty_output_progress::ReaderBackedPtyOutput::for_release_runtime_test(
                master.as_ref().unwrap().as_ref(),
                Arc::clone(&fixture.state.reader_lifecycle),
                Arc::clone(&fixture.state.child_waited),
                Arc::clone(&fixture.state.pty_alias_custody),
                authority,
            )
            .unwrap();
        drop(master);
        let completion = Self::spawn_responder(fixture);
        (
            proof,
            output,
            Self {
                completion: Some(completion),
            },
        )
    }
}
