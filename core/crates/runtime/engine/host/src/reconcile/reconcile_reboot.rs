impl Host {
    pub(crate) fn settle_reboot_migration(&self, terminal_id: Uuid) -> bool {
        match crate::durable_reboot::settle_pending_session(
            &self.paths,
            &self.namespace,
            terminal_id,
        ) {
            Ok(_) => true,
            Err(error) => {
                tracing::error!(
                    %terminal_id,
                    %error,
                    "durable reboot settlement failed during reconciliation"
                );
                false
            }
        }
    }
}
