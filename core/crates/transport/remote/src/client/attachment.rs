impl RemoteAttachment {
    pub async fn next_event(&mut self) -> Result<EventEnvelope, RemoteError> {
        self.rpc
            .next_event(self.terminal_id, self.runtime_epoch)
            .await
    }

    pub async fn detach(mut self) -> Result<(), RemoteError> {
        self.rpc.shutdown_until(RemoteDeadline::operation()).await
    }
}
