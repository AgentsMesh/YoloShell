use super::*;

impl Host {
    pub(crate) async fn handle_client(self: Arc<Self>, stream: UnixStream) -> Result<()> {
        rpc::serve(self, stream).await
    }

    pub(crate) fn verify_namespace(&self) -> Result<()> {
        self.namespace.verify()?;
        self.socket_namespace.verify()
    }
}
