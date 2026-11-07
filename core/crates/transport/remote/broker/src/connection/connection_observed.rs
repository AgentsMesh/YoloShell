impl ObservedConnection {
    pub(super) fn connection(&self) -> &Arc<dyn MultiplexedSshConnection> {
        &self.connection
    }
    pub(super) fn id(&self) -> Uuid {
        self.id
    }
    pub(super) fn generation(&self) -> u64 {
        self.generation
    }
}
