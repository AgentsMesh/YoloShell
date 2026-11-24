struct HubTasks {
    capacity: Arc<Semaphore>,
    proxies: BrokerTasks,
}

impl HubTasks {
    fn new() -> Self {
        Self {
            capacity: Arc::new(Semaphore::new(super::hub_peer_constants::MAX_HUB_TUNNELS)),
            proxies: BrokerTasks::default(),
        }
    }
    fn promote(&self, established: super::hub_peer_proxy::EstablishedHubPeer) {
        self.proxies.spawn(async move {
            contain_task(async move {
                if let Err(error) = established.forward().await {
                    report_task_error(error, super::hub_peer_constants::HUB_TUNNEL_CLOSED);
                }
            })
            .await;
        });
    }
}
