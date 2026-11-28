const RETAINED_CLEANUP_IDENTITY_DUPLICATE_DETAIL: &str =
    "gated PTY owner retained a duplicate cleanup identity";

impl GatedOwner {
    pub(super) fn retain_cleanup_identity(
        &mut self,
        identity: process::ProcessIdentity,
    ) -> process::LiveProcessIdentity {
        assert!(
            !self
                .cleanup_identities
                .iter()
                .any(|(known, _)| *known == identity),
            "{RETAINED_CLEANUP_IDENTITY_DUPLICATE_DETAIL}: {identity:?}"
        );
        let retained = process::LiveProcessIdentity::capture_for_test(identity)
            .unwrap()
            .unwrap();
        self.cleanup_identities.push((identity, retained.clone()));
        retained
    }

    fn cleanup_exact_processes(&self) {
        let _ = force_test_process_tree(self.root, &self.root_live);
        for (_, retained) in &self.cleanup_identities {
            let _ = retained.terminate_retained_fixture();
        }
    }
}

impl Drop for GatedOwner {
    fn drop(&mut self) {
        let _ = self.handle.submit_termination(true);
        self.release();
        self.cleanup_exact_processes();
        if let Some(thread) = self.thread.take() {
            let completed = self
                .completed
                .as_ref()
                .is_some_and(|completed| completed.recv_timeout(OWNER_THREAD_JOIN_TIMEOUT).is_ok());
            if completed {
                let _ = thread.join();
            }
        }
        self.cleanup_exact_processes();
    }
}
