impl RemoteBroker {
    pub fn with_authorized_factory(
        config: BrokerConfig,
        startup: AuthorizedBrokerStartup,
        factory: Arc<dyn BrokerConnectionFactory>,
    ) -> Self {
        let (paths, authority, lock, deadline) = startup.into_parts();
        Self::finish(config, paths, factory, Some((authority, lock, deadline)))
    }
}
