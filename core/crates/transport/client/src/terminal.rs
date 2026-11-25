use super::*;

impl Client {
    pub async fn acquire_writer(
        &self,
        target: RuntimeRef,
        holder_id: impl Into<String>,
        ttl_ms: Option<u64>,
    ) -> Result<WriterLeaseGrant, ClientError> {
        self.acquire_writer_for_instance(
            target,
            holder_id,
            None,
            WriterCapability::generate(),
            ttl_ms,
        )
        .await
    }

    pub async fn acquire_writer_for_instance(
        &self,
        target: RuntimeRef,
        holder_id: impl Into<String>,
        controller_instance_id: Option<String>,
        writer_capability: WriterCapability,
        ttl_ms: Option<u64>,
    ) -> Result<WriterLeaseGrant, ClientError> {
        self.request(
            method::ACQUIRE_WRITER,
            &LeaseRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                holder_id: holder_id.into(),
                ttl_ms,
                generation: None,
                controller_instance_id,
                writer_capability,
            },
        )
        .await
    }

    pub async fn take_over_writer(
        &self,
        target: RuntimeRef,
        holder_id: impl Into<String>,
        ttl_ms: Option<u64>,
    ) -> Result<WriterLeaseGrant, ClientError> {
        self.take_over_writer_for_instance(
            target,
            holder_id,
            None,
            None,
            WriterCapability::generate(),
            ttl_ms,
        )
        .await
    }

    pub async fn take_over_writer_for_instance(
        &self,
        target: RuntimeRef,
        holder_id: impl Into<String>,
        controller_instance_id: Option<String>,
        observed_generation: Option<u64>,
        writer_capability: WriterCapability,
        ttl_ms: Option<u64>,
    ) -> Result<WriterLeaseGrant, ClientError> {
        self.request(
            method::TAKEOVER_WRITER,
            &LeaseRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                holder_id: holder_id.into(),
                ttl_ms,
                generation: observed_generation,
                controller_instance_id,
                writer_capability,
            },
        )
        .await
    }

    pub async fn renew_writer(
        &self,
        grant: &WriterLeaseGrant,
        target: RuntimeRef,
        ttl_ms: Option<u64>,
    ) -> Result<WriterLeaseGrant, ClientError> {
        self.request(
            method::RENEW_WRITER,
            &LeaseRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                holder_id: grant.lease.holder_id.clone(),
                ttl_ms,
                generation: Some(grant.lease.generation),
                controller_instance_id: grant.lease.controller_instance_id.clone(),
                writer_capability: grant.writer_capability.clone(),
            },
        )
        .await
    }

    pub async fn release_writer(
        &self,
        grant: &WriterLeaseGrant,
        target: RuntimeRef,
    ) -> Result<(), ClientError> {
        self.request(
            method::RELEASE_WRITER,
            &LeaseRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                holder_id: grant.lease.holder_id.clone(),
                ttl_ms: None,
                generation: Some(grant.lease.generation),
                controller_instance_id: grant.lease.controller_instance_id.clone(),
                writer_capability: grant.writer_capability.clone(),
            },
        )
        .await
    }

    pub async fn get_writer_lease(
        &self,
        target: RuntimeRef,
    ) -> Result<Option<WriterLease>, ClientError> {
        self.request(method::GET_WRITER_LEASE, &target).await
    }

    pub async fn input(
        &self,
        target: RuntimeRef,
        generation: u64,
        writer_capability: &WriterCapability,
        data: impl Into<Vec<u8>>,
    ) -> Result<InputResult, ClientError> {
        self.request(
            method::TERMINAL_INPUT,
            &InputRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                generation,
                writer_capability: writer_capability.clone(),
                input_id: Uuid::new_v4(),
                data: data.into(),
            },
        )
        .await
    }

    pub async fn input_with_id(&self, request: &InputRequest) -> Result<InputResult, ClientError> {
        self.request(method::TERMINAL_INPUT, request).await
    }

    pub async fn resize(
        &self,
        target: RuntimeRef,
        generation: u64,
        writer_capability: &WriterCapability,
        cols: u16,
        rows: u16,
    ) -> Result<(), ClientError> {
        self.request(
            method::RESIZE_TERMINAL,
            &ResizeRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                generation,
                writer_capability: writer_capability.clone(),
                cols,
                rows,
            },
        )
        .await
    }

    pub async fn signal(
        &self,
        target: RuntimeRef,
        generation: u64,
        writer_capability: &WriterCapability,
        signal: impl Into<String>,
    ) -> Result<(), ClientError> {
        self.request(
            method::SIGNAL_SESSION,
            &SignalRequest {
                terminal_id: target.terminal_id,
                runtime_epoch: target.runtime_epoch,
                generation,
                writer_capability: writer_capability.clone(),
                signal: signal.into(),
            },
        )
        .await
    }
}
