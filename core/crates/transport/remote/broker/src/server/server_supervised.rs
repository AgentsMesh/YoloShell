impl RemoteBroker {
    pub async fn serve_supervised_until<F>(
        self,
        adoption: tokio::sync::oneshot::Receiver<super::BrokerAdoptionAcknowledgement>,
        shutdown: F,
    ) -> Result<()>
    where
        F: Future<Output = ()> + Send,
    {
        self.serve_until_with_role_limits(
            shutdown,
            Some(adoption),
            MAX_RUNTIME_CLIENTS,
            MAX_RUNTIME_ATTACHMENTS,
            MAX_MANAGEMENT_CLIENTS,
        )
        .await
    }
}
