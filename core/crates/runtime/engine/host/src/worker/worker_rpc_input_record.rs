impl Host {
    pub(crate) async fn runtime_record_for_input(
        &self,
        terminal_id: Uuid,
    ) -> std::result::Result<RuntimeRecord, ErrorBody> {
        let record: RuntimeRecord = self
            .read_namespace_json(&self.paths.runtime_record(terminal_id))
            .map_err(|_| {
                ErrorBody::new(
                    crate::protocol::error_code::WORKER_UNREACHABLE,
                    "session has no live runtime record",
                    true,
                )
            })?;
        if !runtime_loss::runtime_record_identity_is_verifiable(&record, terminal_id) {
            return Err(ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                "runtime record cannot establish process identity",
                false,
            ));
        }
        validate_runtime_bootstrap(record.bootstrap_delivery)?;
        if process::boot_identity_relation(&record.boot_id, &self.boot_id)
            == process::BootIdentityRelation::Different
        {
            return Err(ErrorBody::new(
                crate::protocol::error_code::HOST_REBOOTED,
                "runtime belongs to a previous host boot",
                false,
            ));
        }
        Ok(record)
    }
}
