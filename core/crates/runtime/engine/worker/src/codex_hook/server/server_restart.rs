impl CodexHookServer {
    pub(in crate::worker) async fn restart(&mut self, namespace: &WorkerNamespace) -> Result<()> {
        let directory = socket_directory(namespace, &self.path)?;
        self.restart_in(directory).await
    }

    pub(super) async fn restart_retained(&mut self) -> Result<()> {
        let directory = self.socket_directory.try_clone()?;
        self.restart_in(directory).await
    }

    async fn restart_in(
        &mut self,
        directory: crate::paths::directory::PrivateDirectory,
    ) -> Result<()> {
        let (directory, publication_directory) = retain_socket_directory(directory)?;
        self.listener.take();
        self.publication.take();
        let (listener, publication) = publish(publication_directory, &self.path).await?;
        verify_socket_directory(&directory)?;
        self.socket_directory = directory;
        self.listener = Some(listener);
        self.publication = Some(publication);
        Ok(())
    }
}
