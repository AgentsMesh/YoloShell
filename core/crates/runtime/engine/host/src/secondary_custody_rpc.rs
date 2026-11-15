use super::*;

impl Host {
    pub(super) fn rpc_custody_ready(&self, record: &RuntimeRecord) -> bool {
        #[cfg(test)]
        if self.secondary_custody.rpc_test_authorized(record) {
            return true;
        }
        self.secondary_custody.ready_for(
            record,
            self.secondary_custody_holder,
            self.controller_generation,
        )
    }

    pub(super) async fn require_rpc_custody(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<(), WorkerRpcError> {
        #[cfg(test)]
        if self.secondary_custody.rpc_test_authorized(record) {
            return Ok(());
        }
        self.ensure_secondary_custody(record).await
    }

    pub(super) async fn custodied_worker_stream(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<crate::worker::WorkerConnection, WorkerRpcError> {
        Ok(self.custodied_worker_stream_with_hello(record).await?.0)
    }

    pub(super) async fn custodied_worker_stream_with_hello(
        &self,
        record: &RuntimeRecord,
    ) -> std::result::Result<
        (crate::worker::WorkerConnection, crate::model::WorkerHello),
        WorkerRpcError,
    > {
        self.require_rpc_custody(record).await?;
        #[cfg(test)]
        if let Some(peer) = self.secondary_custody.authorized_peer_for_test(record) {
            return crate::worker::authenticated_stream_with_hello_for_host_test(
                record,
                self.controller_generation,
                crate::worker::process::ProcessIdentity {
                    pid: peer.pid,
                    start_time: peer.start_time,
                },
            )
            .await;
        }
        crate::worker::authenticated_stream_with_hello(record, self.controller_generation).await
    }

    pub(super) async fn authenticated_worker_rpc<T: Serialize>(
        &self,
        record: &RuntimeRecord,
        method: &str,
        payload: &T,
    ) -> std::result::Result<(ResponseEnvelope, crate::model::WorkerHello), WorkerRpcError> {
        let mut slot = self.worker_connections.lock(record.terminal_id).await;
        let stale = slot
            .as_ref()
            .is_some_and(|entry| !worker_connections::identity_matches(entry, record));
        if stale {
            *slot = None;
        }
        if slot.is_none() {
            let (connection, hello) = self.custodied_worker_stream_with_hello(record).await?;
            *slot = Some(worker_connections::WorkerConnectionEntry {
                record: record.clone(),
                hello,
                connection,
            });
        }
        let entry = slot.as_mut().expect("worker connection inserted");
        let request = RequestEnvelope::new(
            Uuid::new_v4().to_string(),
            method,
            crate::protocol::to_value(payload).map_err(WorkerRpcError::protocol)?,
        );
        match exchange_worker_request(&mut entry.connection, &request).await {
            Ok(response) => Ok((response, entry.hello.clone())),
            Err(error) => {
                if error.is_transport() || matches!(error, WorkerRpcError::OutcomeUnknown) {
                    *slot = None;
                }
                Err(error)
            }
        }
    }

    pub(super) async fn custodied_worker_rpc<T: Serialize>(
        &self,
        record: &RuntimeRecord,
        method: &str,
        payload: &T,
    ) -> std::result::Result<ResponseEnvelope, WorkerRpcError> {
        self.require_rpc_custody(record).await?;
        let (response, _) = self
            .authenticated_worker_rpc(record, method, payload)
            .await?;
        Ok(response)
    }
}
