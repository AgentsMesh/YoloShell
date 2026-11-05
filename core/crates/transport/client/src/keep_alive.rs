use super::{
    Client, HostStatus, RpcConnection, constants::HOST_KEEPALIVE_INTERVAL, protocol::method,
};
use tokio::{
    sync::watch,
    time::{MissedTickBehavior, interval},
};

impl Client {
    pub async fn retain_host(&self) -> Result<HostHold, super::ClientError> {
        let mut connection = self.connect().await?;
        let status = match connection.take_authenticated_status() {
            Some(status) => status,
            None => connection.request(method::HOST_STATUS, &()).await?,
        };
        let (stop, stopped) = watch::channel(false);
        let client = self.clone();
        let task = tokio::spawn(async move {
            client.hold_connection(Some(connection), stopped).await;
        });
        Ok(HostHold { status, stop, task })
    }

    /// Retains an authenticated control connection while its owner is active.
    /// Reconnection only inspects status; it never retries a session mutation.
    pub async fn hold_host(&self, stop: watch::Receiver<bool>) {
        self.hold_connection(None, stop).await;
    }

    async fn hold_connection(
        &self,
        mut connection: Option<RpcConnection>,
        mut stop: watch::Receiver<bool>,
    ) {
        let mut tick = interval(HOST_KEEPALIVE_INTERVAL);
        tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            if *stop.borrow() {
                break;
            }
            tokio::select! { _ = stop.changed() => break, _ = tick.tick() => {} }
            if connection.is_none() {
                connection = self.connect().await.ok();
            }
            if let Some(current) = connection.as_mut() {
                if current
                    .request::<_, HostStatus>(method::HOST_STATUS, &())
                    .await
                    .is_err()
                {
                    connection = None;
                }
            }
        }
    }
}

pub struct HostHold {
    status: HostStatus,
    stop: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl HostHold {
    pub fn initial_host_pid(&self) -> u32 {
        self.status.host_pid
    }
}

impl Drop for HostHold {
    fn drop(&mut self) {
        self.stop.send_replace(true);
        self.task.abort();
    }
}
