use super::*;
use std::sync::atomic::AtomicUsize;

const ONE_CALL: usize = 1;

#[derive(Default)]
pub struct Calls {
    connections: AtomicUsize,
    runtime: AtomicUsize,
    commands: AtomicUsize,
    deployments: AtomicUsize,
}
impl Calls {
    pub(super) fn runtime(&self) {
        self.runtime.fetch_add(ONE_CALL, Ordering::SeqCst);
    }
    pub(super) fn command(&self) {
        self.commands.fetch_add(ONE_CALL, Ordering::SeqCst);
    }
    pub(super) fn deployment(&self) {
        self.deployments.fetch_add(ONE_CALL, Ordering::SeqCst);
    }
    pub fn assert_ssh_only(&self, connections: usize) {
        assert_eq!(self.connections.load(Ordering::SeqCst), connections);
        assert_eq!(self.runtime.load(Ordering::SeqCst), usize::default());
        assert_eq!(self.commands.load(Ordering::SeqCst), usize::default());
        assert_eq!(self.deployments.load(Ordering::SeqCst), usize::default());
    }
}

struct CountingFactory(Arc<Calls>);
impl BrokerConnectionFactory for CountingFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        self.0.connections.fetch_add(ONE_CALL, Ordering::SeqCst);
        let mut connection = ReadyConnection::new();
        connection.calls = self.0.clone();
        Box::pin(async move { Ok(Arc::new(connection) as Arc<dyn MultiplexedSshConnection>) })
    }
}

pub fn counted_factory() -> (Arc<dyn BrokerConnectionFactory>, Arc<Calls>) {
    let calls = Arc::new(Calls::default());
    (Arc::new(CountingFactory(calls.clone())), calls)
}
